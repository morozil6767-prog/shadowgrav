//! Simulation thread: owns the `World`, applies commands BETWEEN steps, publishes immutable snapshots.
//! No physics lives here: every number below comes from shadowgrav-core (advance, snapshot, driver helpers).
use crate::presets::{self, Meta};
use crate::protocol::*;
use shadowgrav_core::diagnostics::{
    delta_na, pair_effective_g, reference_diag, sky_and_heating, snapshot, tau_max, EnergyLabel,
};
use shadowgrav_core::driver::screening_ratio;
use shadowgrav_core::geom::{norm, sub};
use shadowgrav_core::skymap::{sky_deficit_map, SkyFocus};
use shadowgrav_core::state::{StepControl, World};
use shadowgrav_core::{advance, Kind, Mode, Validation};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type Slot = Arc<Mutex<Option<Arc<FrameSnapshot>>>>;
pub struct SimHandle {
    pub tx: Sender<Command>,
    pub latest: Slot,
}

const STEP_BUDGET: Duration = Duration::from_millis(8);
const PUBLISH_EVERY: Duration = Duration::from_millis(16);
const DIAG_EVERY: Duration = Duration::from_millis(100);

/// `wake` is called after every publish (the UI passes `ctx.request_repaint()`).
pub fn spawn(wake: Box<dyn Fn() + Send>) -> SimHandle {
    let (tx, rx) = mpsc::channel();
    let latest: Slot = Arc::new(Mutex::new(None));
    let slot = latest.clone();
    std::thread::Builder::new()
        .name("sim".into())
        .spawn(move || run(rx, slot, wake))
        .expect("spawn sim thread");
    SimHandle { tx, latest }
}

struct Sim {
    world: World,
    meta: Meta,
    spec: Option<PresetSpec>,
    running: bool,
    rate: f64,
    acc: f64,
    error: Option<String>,
    seq: u64,
    load_gen: u64,
    diag_seq: u64,
    diag: Arc<DiagView>,
    last_diag: Instant,
    sps: f64,
    sps_t: Instant,
    sps_n: u64,
    dirty: bool,
    sky_on: bool,
    sky_target: usize,
    sky_focus: SkyFocus,
    sky_fov: f64,
    /// Opt-in (env SHADOWGRAV_EXTRA_DIAG=1): reference error and tau_max in the slow diagnostics.
    extra_diag: bool,
}

fn run(rx: Receiver<Command>, slot: Slot, wake: Box<dyn Fn() + Send>) {
    let spec = PresetSpec::default_for(PresetKind::TwoBodyOrbit);
    let built = presets::build(&spec).expect("default preset must build");
    let now = Instant::now();
    let mut s = Sim {
        world: built.world,
        meta: built.meta,
        spec: Some(spec),
        running: false,
        rate: 600.0,
        acc: 0.0,
        error: None,
        seq: 0,
        load_gen: 0,
        diag_seq: 0,
        diag: Arc::new(DiagView::default()),
        last_diag: now - DIAG_EVERY,
        sps: 0.0,
        sps_t: now,
        sps_n: 0,
        dirty: true,
        sky_on: true,
        sky_target: 0,
        sky_focus: SkyFocus::All,
        sky_fov: 4.0,
        // Default ON: reference + tau_max for small N make the instrument self-auditing.
        // Set SHADOWGRAV_EXTRA_DIAG=0 to disable.
        extra_diag: std::env::var("SHADOWGRAV_EXTRA_DIAG")
            .map(|v| v != "0" && !v.is_empty())
            .unwrap_or(true),
    };
    let (mut last_tick, mut last_pub) = (now, now - PUBLISH_EVERY);
    loop {
        // ---- commands (blocking only while paused) ----
        if !s.running {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(c) => s.handle(c),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        loop {
            match rx.try_recv() {
                Ok(c) => s.handle(c),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        // ---- stepping ----
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;
        if s.running {
            let due: u64 = if s.rate > 0.0 {
                s.acc += s.rate * dt.min(0.1);
                s.acc.floor() as u64
            } else {
                u64::MAX
            };
            let t0 = Instant::now();
            let mut done = 0u64;
            while done < due && t0.elapsed() < STEP_BUDGET {
                if !s.step_once() {
                    break;
                }
                done += 1;
            }
            if s.rate > 0.0 {
                s.acc = (s.acc - done as f64).clamp(0.0, 0.05 * s.rate);
            }
            if done > 0 {
                s.dirty = true;
            }
            if s.rate > 0.0 && done == due {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        // ---- throughput estimate ----
        let el = s.sps_t.elapsed().as_secs_f64();
        if el >= 0.5 {
            s.sps = s.sps_n as f64 / el;
            s.sps_n = 0;
            s.sps_t = Instant::now();
            s.dirty = true;
        }
        // ---- publish ----
        if s.dirty && last_pub.elapsed() >= PUBLISH_EVERY {
            let frame = s.frame();
            *slot.lock().unwrap() = Some(Arc::new(frame));
            last_pub = Instant::now();
            s.dirty = false;
            wake();
        }
    }
}

impl Sim {
    fn step_once(&mut self) -> bool {
        match advance(&mut self.world) {
            Ok(_) => {
                self.sps_n += 1;
                true
            }
            Err(e) => {
                self.error = Some(format!("{e:?}"));
                self.running = false;
                self.dirty = true;
                false
            }
        }
    }

    fn cur_h(&self) -> f64 {
        match self.world.cfg.step {
            StepControl::Fixed { h } => h,
            StepControl::Adaptive { h_max, .. } => h_max,
        }
    }

    fn report<T>(&mut self, r: Result<T, shadowgrav_core::SimError>) {
        match r {
            Ok(_) => self.error = None,
            Err(e) => self.error = Some(format!("{e:?} (change rolled back)")),
        }
    }

    fn handle(&mut self, c: Command) {
        self.dirty = true;
        match c {
            Command::Play => {
                if self.error.is_none() {
                    self.running = true;
                }
            }
            Command::Pause => self.running = false,
            Command::Step(n) => {
                self.running = false;
                for _ in 0..n {
                    if !self.step_once() {
                        break;
                    }
                }
            }
            Command::Load(spec) => match presets::build(&spec) {
                Ok(b) => {
                    self.world = b.world;
                    self.meta = b.meta;
                    self.spec = Some(spec);
                    self.running = false;
                    self.error = None;
                    self.load_gen += 1;
                    self.acc = 0.0;
                    self.last_diag = Instant::now() - DIAG_EVERY;
                }
                Err(e) => self.error = Some(format!("preset failed: {e:?}")),
            },
            Command::SetRate(r) => self.rate = r.max(0.0),
            Command::SetFixedH(h) => {
                let r = self.world.set_step_control(StepControl::Fixed { h });
                self.report(r);
            }
            Command::SetAdaptive(on) => {
                let h = self.cur_h();
                let ctl = if on {
                    StepControl::Adaptive {
                        h_max: h,
                        eta_t: 0.005,
                    }
                } else {
                    StepControl::Fixed { h }
                };
                let r = self.world.set_step_control(ctl);
                self.report(r);
            }
            Command::SetMode(m) => {
                let r = self.world.apply_options(|o| o.mode = m);
                self.report(r);
            }
            Command::SetKappa0(v) => {
                let r = self.world.apply_globals(|g| g.kappa0 = v);
                self.report(r);
            }
            Command::SetXi(v) => {
                let r = self.world.apply_globals(|g| g.xi = v);
                self.report(r);
            }
            Command::SetVg(v) => {
                let r = self.world.apply_globals(|g| g.v_g = v);
                self.report(r);
            }
            Command::SetAlphaBg(v) => {
                let r = self.world.apply_globals(|g| g.alpha_bg = v);
                self.report(r);
            }
            Command::SetF(v) => {
                let r = self.world.apply_globals(|g| g.f = v);
                self.report(r);
            }
            Command::SetQuadTier(tier) => {
                let r = self.world.apply_options(|o| {
                    o.validation = match tier {
                        QuadTier::Standard => None,
                        QuadTier::Validation => Some(Validation::default_validation()),
                        QuadTier::Fine => Some(Validation::xhi()),
                        QuadTier::Certified => Some(Validation::certified()),
                    };
                });
                self.report(r);
            }
            Command::SetInstrumented(on) => {
                let r = self.world.apply_options(|o| o.instrumented = on);
                self.report(r);
            }
            Command::SetParallel(on) => {
                let r = self.world.apply_options(|o| o.parallel = on);
                self.report(r);
            }
            Command::SetThinFast(on) => {
                let r = self.world.apply_options(|o| o.thin_fast = on);
                self.report(r);
            }
            Command::SetSkyTarget(k) => self.sky_target = k,
            Command::SetSkyFocus(f, fov) => {
                self.sky_focus = f;
                self.sky_fov = fov.max(1.0);
            }
            Command::SetSkyEnabled(on) => self.sky_on = on,
            Command::SetLeafBudget(b) => {
                let r = self.world.apply_options(|o| o.leaf_budget = b);
                self.report(r);
            }
        }
        if self.error.is_some() {
            self.running = false;
        }
        self.last_diag = Instant::now() - DIAG_EVERY; // parameter edits refresh the diagnostics immediately
    }

    fn frame(&mut self) -> FrameSnapshot {
        if self.last_diag.elapsed() >= DIAG_EVERY {
            self.diag_seq += 1;
            self.diag = Arc::new(self.build_diag());
            self.last_diag = Instant::now();
        }
        self.seq += 1;
        let w = &self.world;
        let bodies = (0..w.n())
            .map(|i| {
                let probe = w.p.kind[i] == Kind::Probe;
                BodyView {
                    x: w.p.x[i],
                    v: w.p.v[i],
                    mass: w.p.m[i],
                    radius: if probe { 0.0 } else { w.dv.a[i] },
                    tau_c: w.dv.tau_c[i],
                    s: w.dv.s[i],
                    is_probe: probe,
                    f_total: w.force.f_total[i],
                    f_pair: w.force.f_pair[i],
                    f_corr: w.force.f_corr[i],
                    f_dr: w.force.f_dr[i],
                    f_ab: w.force.f_ab[i],
                }
            })
            .collect();
        let params = ParamView {
            mode: w.opt.mode,
            kappa0: w.g.kappa0,
            xi: w.g.xi,
            v_g: w.g.v_g,
            alpha_bg: w.g.alpha_bg,
            g_n: w.g.g_n,
            pi_flux: w.g.pi_flux(),
            calibrated: w.g.pi_override.is_none(),
            h: self.cur_h(),
            adaptive: matches!(w.cfg.step, StepControl::Adaptive { .. }),
            f: w.g.f,
            quad_tier: match &w.opt.validation {
                None => QuadTier::Standard,
                Some(v) if v.f_v <= 7.5e-4 => QuadTier::Certified,
                Some(v) if v.f_v <= 0.0025 => QuadTier::Fine,
                Some(_) => QuadTier::Validation,
            },
            instrumented: w.opt.instrumented,
            leaf_budget: w.opt.leaf_budget,
            ell_cap: w.g.ell_cap,
            thin_fast: w.opt.thin_fast,
            parallel: w.opt.parallel,
        };
        let (badge, badge_reasons, badge_n_red) = self.badge();
        FrameSnapshot {
            seq: self.seq,
            load_gen: self.load_gen,
            preset: self.spec,
            running: self.running,
            rate: self.rate,
            steps_per_sec: self.sps,
            time: w.time(),
            step: w.step_count,
            bodies,
            params,
            diag: self.diag.clone(),
            badge,
            badge_reasons,
            badge_n_red,
            error: self.error.clone(),
        }
    }

    /// Conformance badge (Architecture section 7). Red = result not at contract quality; amber = labelled caveat.
    fn badge(&self) -> (Badge, Vec<String>, usize) {
        let w = &self.world;
        let (mut red, mut amber) = (Vec::new(), Vec::new());
        if let Some(e) = &self.error {
            red.push(format!("engine error: {e}"));
        }
        if w.flags.cap_hit_total > 0 {
            red.push("cap-hit: HEALPix refinement cap reached".to_string());
        }
        if w.flags.fp_nonconverged_steps > 0 {
            red.push("eq. 12 fixed point not converged on some steps".to_string());
        }
        if w.opt.thin_fast {
            red.push("thin-fast preview: NON-CONFORMING (Barnes-Hut monopole, no overlap correction, uniform-sphere field)".to_string());
        }
        if w.opt.mode == Mode::V2 {
            if !w.force.status.v2_valid {
                red.push("V2 gating: max |eps| > 0.1".to_string());
            } else {
                amber.push(
                    "V2: aberration (eq. 10 verified to O(eps^2); multi-step V2 less tested)"
                        .to_string(),
                );
            }
        }
        if w.flags.adaptive_step_used {
            amber.push("adaptive h: non-reversible, non-symplectic".to_string());
        }
        if w.g.pi_override.is_some() {
            amber.push("uncalibrated mode (Pi overridden)".to_string());
        }
        if w.ledger.ever_nonconservative {
            amber.push("energy: diagnostic, non-conservative regime".to_string());
        }
        if !red.is_empty() {
            let nr = red.len();
            red.extend(amber);
            (Badge::Red, red, nr)
        } else if !amber.is_empty() {
            (Badge::Amber, amber, 0)
        } else {
            (
                Badge::Green,
                vec!["conforming configuration (V0/V1 core path, calibrated, fixed h)".to_string()],
                0,
            )
        }
    }

    fn build_diag(&self) -> DiagView {
        let w = &self.world;
        let cs = snapshot(w);
        let bodies: Vec<usize> = (0..w.n()).filter(|&i| w.p.kind[i] == Kind::Body).collect();
        let (mut tmin, mut tmax, mut tsum, mut smin, mut ssum) =
            (f64::INFINITY, 0.0f64, 0.0, f64::INFINITY, 0.0);
        for &i in &bodies {
            let (t, s) = (w.dv.tau_c[i], w.dv.s[i]);
            tmin = tmin.min(t);
            tmax = tmax.max(t);
            tsum += t;
            smin = smin.min(s);
            ssum += s;
        }
        let nb = bodies.len().max(1) as f64;
        let ls = w.last_step;
        let mut d = DiagView {
            seq: self.diag_seq,
            e: cs.conservation.e,
            ke: cs.conservation.ke,
            pe: cs.conservation.pe,
            energy_nonconservative: matches!(
                cs.conservation.label,
                EnergyLabel::DiagnosticNonConservative
            ),
            de: cs.ledger.de,
            max_de: cs.ledger.max_de,
            dp: cs.ledger.dp,
            max_dp: cs.ledger.max_dp,
            dl: cs.ledger.dl,
            max_dl: cs.ledger.max_dl,
            rp_shadow: cs.ledger.rp_shadow,
            rp_total: cs.ledger.rp_total,
            max_rp_total: cs.ledger.max_rp_total,
            targets: cs
                .targets
                .iter()
                .map(|t| TargetRow {
                    k: t.k,
                    delta_na: t.delta_na,
                    delta_sat: t.delta_sat,
                    r_newton: t.r_newton,
                    r_thin: t.r_thin,
                    gamma: t.gamma_drag,
                })
                .collect(),
            pairs: pair_effective_g(w)
                .unwrap_or_default()
                .into_iter()
                .map(|p| PairRow {
                    k: p.k,
                    j: p.j,
                    g_eff: p.g_eff,
                    expected: p.expected,
                })
                .collect(),
            leaf_count: cs.status.leaf_count,
            cap_hits: cs.status.cap_hit_count,
            max_corr_over_f: cs.status.max_corr_over_f,
            fp_iters: ls.map(|s| s.fp_iters).unwrap_or(0),
            fp_residual: ls.map(|s| s.fp_residual).unwrap_or(0.0),
            fp_converged: ls.map(|s| s.fp_converged).unwrap_or(true),
            force_evals: ls.map(|s| s.force_evals).unwrap_or(0),
            tau_min: if tmin.is_finite() { tmin } else { 0.0 },
            tau_mean: tsum / nb,
            tau_max: tmax,
            s_min: if smin.is_finite() { smin } else { 1.0 },
            s_mean: ssum / nb,
            thin_valid: cs.status.thin_valid,
            v2_valid: cs.status.v2_valid,
            overlap_present: cs.status.overlap_present,
            interior_present: cs.status.interior_target_present,
            labels: cs.labels.iter().map(|s| s.to_string()).collect(),
            extras: Vec::new(),
            sky: None,
            sky_note: None,
            d_bar_eq20: None,
            ref_check: None,
            tau_add_max: None,
        };
        if w.opt.thin_fast {
            d.labels.push("thin-fast: NON-CONFORMING".to_string());
        }
        d.extras = self.extras(&d);
        if self.extra_diag {
            self.extra_diagnostics(&mut d);
        }
        if self.sky_on {
            let kk = if bodies.contains(&self.sky_target) {
                self.sky_target
            } else {
                bodies.first().copied().unwrap_or(0)
            };
            match sky_deficit_map(w, kk, self.sky_focus, self.sky_fov) {
                Ok(m) => d.sky = Some(m),
                Err(e) => d.sky_note = Some(e),
            }
            d.d_bar_eq20 = sky_and_heating(w, kk).map(|h| h.d_bar);
        } else {
            d.sky_note = Some("sky map switched off".to_string());
        }
        d
    }

    /// Opt-in extras (workspace_9, uncompiled): Spec 6 item 6 tau_max lower bound and Spec 6 item 11 reference error, shown as
    /// read-out rows so no egui change is needed. The reference is an all-rays integral, so the live view skips it for N > 8.
    fn extra_diagnostics(&self, d: &mut DiagView) {
        let w = &self.world;
        let tmax = tau_max(w).iter().fold(0.0f64, |m, t| m.max(t.lower_bound));
        d.tau_add_max = Some(tmax);
        d.extras.push((
            "tau_max (additive, lower bound)".to_string(),
            format!("{tmax:.4e}"),
        ));
        if w.n() > 8 {
            d.extras.push((
                "reference check".to_string(),
                "skipped in the live view: N > 8".to_string(),
            ));
            return;
        }
        let r = reference_diag(w, 50, 100);
        // Full sentence goes to the export / sgrun JSON only. The read-out panel sizes itself to its widest row, so the live view
        // shows short rows (a ~180-character row once pushed the 3D view out of the default layout).
        let text = r.summary();
        match (
            r.max_rel_diff(),
            r.reference_est_err(),
            r.exceeds_reference_resolution(),
        ) {
            (Some(diff), Some(est), Some(exceeds)) => {
                d.extras
                    .push(("all-rays: max rel diff".to_string(), format!("{diff:.2e}")));
                d.extras
                    .push(("  ref. own error".to_string(), format!("{est:.1e}")));
                d.extras.push((
                    "  gap vs ref. error".to_string(),
                    (if exceeds { "exceeds it" } else { "within it" }).to_string(),
                ));
                // For two disjoint spheres the engine uses the exact closed form, so a gap inside the reference error is the reference's.
                if w.n() == 2 && !d.overlap_present && !exceeds {
                    d.extras.push((
                        "  note (disjoint pair)".to_string(),
                        "gap = ref. error".to_string(),
                    ));
                }
            }
            _ => {
                let short: String = text.chars().take(44).collect();
                d.extras.push(("all-rays check".to_string(), short));
            }
        }
        d.ref_check = Some(RefView {
            max_rel_diff: r.max_rel_diff(),
            est_err: r.reference_est_err(),
            exceeds_est: r.exceeds_reference_resolution(),
            text,
        });
    }

    fn extras(&self, d: &DiagView) -> Vec<(String, String)> {
        let w = &self.world;
        let mut v = Vec::new();
        let Some(spec) = self.spec else { return v };
        match spec.kind {
            PresetKind::TwoBodyOrbit | PresetKind::TwoBodyOrbitV2 if w.n() == 2 => {
                v.push(("omega expected".into(), format!("{:.9}", self.meta.omega)));
                v.push(("T expected".into(), format!("{:.9}", self.meta.period)));
                v.push((
                    "orbits completed".into(),
                    format!("{:.3}", w.time() / self.meta.period),
                ));
                v.push((
                    "separation".into(),
                    format!("{:.9}", norm(&sub(&w.p.x[1], &w.p.x[0]))),
                ));
            }
            PresetKind::Screening if w.n() == 3 => {
                let r = screening_ratio(w);
                let err = (r - 0.8).abs();
                v.push(("R = |F_A|/|F_pair|".into(), format!("{r:.8}")));
                v.push(("|R - 0.8|".into(), format!("{err:.3e}")));
                let tier = match &w.opt.validation {
                    None => QuadTier::Standard,
                    Some(v) if v.f_v <= 7.5e-4 => QuadTier::Certified,
                    Some(v) if v.f_v <= 0.0025 => QuadTier::Fine,
                    Some(_) => QuadTier::Validation,
                };
                let (tol, status) = match tier {
                    QuadTier::Standard => (
                        "standard: |R-0.8| <= 5e-3 (provisional Spec clause)".into(),
                        if err <= 5e-3 {
                            "WITHIN Standard"
                        } else {
                            "OUTSIDE Standard"
                        },
                    ),
                    QuadTier::Validation => (
                        "validation (f_v=0.02): expect ~1e-5 on this geometry; not Spec 1e-6"
                            .into(),
                        if err <= 5e-5 {
                            "WITHIN Validation"
                        } else {
                            "OUTSIDE Validation"
                        },
                    ),
                    QuadTier::Fine => (
                        "fine (f_v=0.002): measured ~3.3e-6; still short of Spec 1e-6".into(),
                        if err <= 5e-6 {
                            "WITHIN Fine"
                        } else {
                            "OUTSIDE Fine"
                        },
                    ),
                    QuadTier::Certified => (
                        "certified (f_v=7e-4): Spec target |R-0.8| <= 1e-6 (measured ~4e-7)".into(),
                        if err <= 1e-6 {
                            "WITHIN Certified (Spec 1e-6)"
                        } else {
                            "OUTSIDE Certified — wait for eval or check geometry"
                        },
                    ),
                };
                v.push(("quad tier".into(), tier.label().into()));
                v.push(("tolerance".into(), tol));
                v.push(("regime status".into(), status.into()));
            }
            PresetKind::PartialOverlap | PresetKind::Overlap3D if w.n() == 3 => {
                let na = delta_na(w, 0);
                v.push(("Delta^NA (body A)".into(), format!("{na:.6e}")));
                match presets::oracle_delta_na(spec.kind, spec.tau_c) {
                    Some(o) => {
                        v.push(("Delta^NA sky oracle".into(), format!("{o:.6e}")));
                        v.push((
                            "relative difference".into(),
                            format!("{:.2e}", (na - o) / o),
                        ));
                        let note = if w.opt.validation.is_some() {
                            "Validation/Fine/Certified: expect |rel| ~<= 1% on this geometry"
                        } else {
                            "Standard: coarse on partial overlap (up to several % on Delta^NA)"
                        };
                        let status = if w.opt.validation.is_some() {
                            if ((na - o) / o).abs() <= 0.01 {
                                "WITHIN finer-tier expectation"
                            } else {
                                "OUTSIDE finer-tier expectation"
                            }
                        } else if ((na - o) / o).abs() <= 0.12 {
                            "WITHIN Standard (coarse) expectation"
                        } else {
                            "OUTSIDE Standard expectation"
                        };
                        v.push(("accuracy".into(), note.into()));
                        v.push(("regime status".into(), status.into()));
                    }
                    None => v.push((
                        "sky oracle".into(),
                        "recorded for tau_c = 1 and 5 only".into(),
                    )),
                }
            }
            PresetKind::Drag | PresetKind::DragFrame if w.n() == 1 => {
                let u = norm(&sub(&w.p.v[0], &w.g.w));
                let u_exp = self.meta.u0 * (-self.meta.gamma_ref * w.time()).exp();
                v.push(("gamma ref".into(), format!("{:.6e}", self.meta.gamma_ref)));
                if let Some(t) = d.targets.first() {
                    if let Some(g) = t.gamma {
                        v.push(("gamma live".into(), format!("{g:.6e}")));
                    }
                }
                v.push((
                    "u / u_expected - 1".into(),
                    format!("{:.3e}", u / u_exp - 1.0),
                ));
            }
            _ => {}
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets;

    fn wait_for<F: Fn(&FrameSnapshot) -> bool>(h: &SimHandle, pred: F) -> Arc<FrameSnapshot> {
        let t0 = Instant::now();
        loop {
            if let Some(s) = h.latest.lock().unwrap().clone() {
                if pred(&s) {
                    return s;
                }
            }
            assert!(
                t0.elapsed() < Duration::from_secs(20),
                "timed out waiting for snapshot"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn every_preset_builds_and_steps() {
        for k in PresetKind::ALL {
            let spec = PresetSpec::default_for(k);
            let mut b = presets::build(&spec).unwrap_or_else(|e| panic!("{k:?}: {e:?}"));
            for _ in 0..3 {
                advance(&mut b.world).unwrap_or_else(|e| panic!("{k:?} step: {e:?}"));
            }
        }
    }

    /// The oracle-checked presets reproduce the recorded sky-oracle Delta^NA within the measured accuracy of each mode.
    #[test]
    fn oracle_presets_match_recorded_delta_na() {
        for kind in [PresetKind::PartialOverlap, PresetKind::Overlap3D] {
            for tau in [1.0, 5.0] {
                let spec = PresetSpec {
                    tau_c: tau,
                    ..PresetSpec::default_for(kind)
                };
                let mut b = presets::build(&spec).unwrap();
                let o = presets::oracle_delta_na(kind, tau).expect("oracle value recorded");
                let std_err = (delta_na(&b.world, 0) - o).abs() / o;
                assert!(std_err <= 0.13, "{kind:?} tau {tau} standard: {std_err:e}");
                b.world
                    .apply_options(|op| op.validation = Some(Validation::default_validation()))
                    .unwrap();
                let val_err = (delta_na(&b.world, 0) - o).abs() / o;
                assert!(
                    val_err <= 1e-2,
                    "{kind:?} tau {tau} validation: {val_err:e}"
                );
            }
        }
    }

    #[test]
    fn sim_thread_runs_and_publishes_green_orbit() {
        let h = spawn(Box::new(|| {}));
        let s0 = wait_for(&h, |_| true);
        assert!(!s0.running && s0.bodies.len() == 2);
        h.tx.send(Command::SetRate(0.0)).unwrap();
        h.tx.send(Command::Play).unwrap();
        let s = wait_for(&h, |s| s.step >= 200);
        assert!(s.running && s.time > 0.0);
        assert_eq!(s.badge, Badge::Green, "{:?}", s.badge_reasons);
        h.tx.send(Command::Pause).unwrap();
        let s = wait_for(&h, |s| !s.running);
        // snapshot is immutable and consistent: energy drift bounded (omega h)^2 ~ 4e-5
        assert!(s.diag.max_de < 1e-4, "max dE/E {:e}", s.diag.max_de);
    }

    #[test]
    fn bad_edit_is_rolled_back_and_flagged_red() {
        let h = spawn(Box::new(|| {}));
        let s0 = wait_for(&h, |_| true);
        h.tx.send(Command::SetMode(Mode::V2)).unwrap();
        h.tx.send(Command::SetAlphaBg(0.1)).unwrap(); // V2 requires alpha_bg = 0 (Spec 2.5)
        let s = wait_for(&h, |s| s.error.is_some());
        assert_eq!(s.badge, Badge::Red);
        assert_eq!(s.params.alpha_bg, s0.params.alpha_bg); // edit rolled back
    }

    #[test]
    fn sky_map_follows_target_and_focus_commands() {
        let h = spawn(Box::new(|| {}));
        let _ = wait_for(&h, |_| true);
        h.tx.send(Command::Load(PresetSpec::default_for(
            PresetKind::Screening,
        )))
        .unwrap();
        h.tx.send(Command::SetSkyTarget(0)).unwrap();
        let s = wait_for(&h, |s| s.bodies.len() == 3 && s.diag.sky.is_some());
        assert_eq!(s.diag.sky.as_ref().unwrap().k, 0);
        assert!(s.diag.sky.as_ref().unwrap().focus.is_none());
        h.tx.send(Command::SetSkyFocus(SkyFocus::Occluder(1), 3.0))
            .unwrap();
        let s = wait_for(&h, |s| {
            s.diag.sky.as_ref().is_some_and(|m| m.focus == Some(1))
        });
        assert!(s.diag.sky.as_ref().unwrap().d_max > 0.0);
        h.tx.send(Command::SetSkyEnabled(false)).unwrap();
        let s = wait_for(&h, |s| s.diag.sky.is_none());
        assert!(s.diag.sky_note.is_some());
    }

    #[test]
    fn load_preset_bumps_generation_and_pauses() {
        let h = spawn(Box::new(|| {}));
        let s0 = wait_for(&h, |_| true);
        h.tx.send(Command::Load(PresetSpec::default_for(
            PresetKind::Screening,
        )))
        .unwrap();
        let s = wait_for(&h, |s| s.load_gen > s0.load_gen);
        assert!(!s.running && s.bodies.len() == 3);
        assert!(s.diag.extras.iter().any(|(k, _)| k.starts_with("R =")));
        assert!(s.diag.extras.iter().any(|(k, _)| k == "regime status"));
        assert!(s.diag.extras.iter().any(|(k, _)| k == "quad tier"));
    }
}
