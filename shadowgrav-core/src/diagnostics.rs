//! Diagnostics aggregation (Spec section 6, subset obtainable from ForceOutput + state).
//! No force re-evaluation here. Sink-based quantities (pair G_eff, D_bar, tau_bar, heating)
//! require Options.instrumented = true and return None otherwise.
//!
//! Implemented: 1 (pair/probe effective G), 2 (Newtonian / thin residuals), 3 (Delta^NA, Delta^sat),
//! 4 (heating, eq. 20), 5 (drag rate), 6 (tau_bar; `tau_max` lower bound, workspace_9), 7 (D_bar),
//! 8 (Seeliger paired read-out `seeliger_check`, workspace_9), 9 (eta_EP, `eta_ep`), 10 (E, P, L, drifts, R_P),
//! 11 (quadrature health from Status; brute-force error `reference_diag`, workspace_9), 12 (validity flags from Status).
//! The workspace_9 additions are opt-in (`snapshot_with`, or called directly): `snapshot()` itself still evaluates no force.
//! `seeliger_check` and `reference_diag` DO evaluate forces / the all-rays reference and are therefore never run implicitly.
use crate::geom::*;
use crate::healpix::{cell_area, pix2vec_nest};
use crate::mathx;
use crate::pair::{pair_p, Node, PairSpec};
use crate::reference::{reference_check, ReferenceCheck};
use crate::state::World;
use crate::sum::{Neum, Neum3};
use crate::types::*;
use std::f64::consts::PI;

pub fn body_indices(p: &Particles) -> Vec<usize> {
    (0..p.x.len())
        .filter(|&k| p.kind[k] == Kind::Body)
        .collect()
}
fn safe_ratio(num: f64, den: f64) -> f64 {
    if den > 1e-300 {
        num / den
    } else {
        num
    }
}
fn rel_resid(f: &V3, r: &V3) -> f64 {
    let (n, d) = (norm(&sub(f, r)), norm(r));
    if d > 0.0 {
        n / d
    } else if n == 0.0 {
        0.0
    } else {
        f64::INFINITY
    }
}

// ------------------------------------------------------------------ conservation (3.4)
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EnergyLabel {
    #[default]
    Conserved,
    DiagnosticNonConservative,
}

#[derive(Clone, Debug)]
pub struct Conservation {
    pub ke: f64,
    pub pe: f64,
    pub e: f64,
    pub p: V3,
    pub l: V3,
    pub sum_m_abs_v: f64,
    pub label: EnergyLabel,
}

fn radii_equal(w: &World) -> bool {
    let b = body_indices(&w.p);
    match b.first() {
        None => true,
        Some(&f) => b
            .iter()
            .all(|&k| (w.dv.a[k] - w.dv.a[f]).abs() <= 1e-12 * w.dv.a[f]),
    }
}

/// E is a valid conserved quantity only for reciprocal pair forces: V0, alpha_bg = 0, no sky-disc
/// overlap (a 3-body correction is not derivable from a pair potential), and no interior pair unless
/// all radii are equal. This is stricter than the Spec's parenthetical (flagged for clarification).
pub fn energy_label(w: &World) -> EnergyLabel {
    let st = &w.force.status;
    let reciprocal_interior = !st.interior_target_present || radii_equal(w);
    if w.opt.mode == Mode::V0 && w.g.alpha_bg == 0.0 && !st.overlap_present && reciprocal_interior {
        EnergyLabel::Conserved
    } else {
        EnergyLabel::DiagnosticNonConservative
    }
}

/// int_d^{a_j} Gamma_k P_kj(r) dr, 8 panels x 16-point Gauss-Legendre (P is the same pair_p as the force).
fn interior_potential_integral(
    w: &World,
    k: usize,
    j: usize,
    d: f64,
    scratch: &mut Vec<Node>,
) -> f64 {
    let a = w.dv.a[j];
    let gamma = w.g.xi * w.g.pi_flux() * w.dv.a_eff[k] / (4.0 * PI);
    let panels = 8usize;
    let width = (a - d) / panels as f64;
    let gl = &w.quad.gl16;
    let mut s = Neum::default();
    for pn in 0..panels {
        let lo = d + width * pn as f64;
        let hi = lo + width;
        let (hw, mid) = (0.5 * (hi - lo), 0.5 * (hi + lo));
        for i in 0..gl.x.len() {
            let r = mid + hw * gl.x[i];
            let p = pair_p(
                PairSpec {
                    d: r,
                    a,
                    beta: w.dv.beta[j],
                    alpha: w.g.alpha_bg,
                    a_eff: w.dv.a_eff[j],
                    force_gl: false,
                },
                &w.quad,
                scratch,
            );
            s.add(hw * gl.w[i] * p);
        }
    }
    gamma * s.value()
}

/// Spec 3.4. U_kj = -G m^g_k m^g_j / d for d >= a_j (k<j, occluder = j), and
/// U(d) = U(a_j) - int_d^{a_j} Gamma_k P_kj dr inside. G here is xi Pi kappa0^2/(4 pi), which equals G_N
/// when calibrated (eq. 19) and is the coupling the force actually uses in uncalibrated mode.
pub fn potential_energy(w: &World) -> f64 {
    let g_eff = w.g.xi * w.g.pi_flux() * w.g.kappa0 * w.g.kappa0 / (4.0 * PI);
    let b = body_indices(&w.p);
    let mut s = Neum::default();
    let mut scratch: Vec<Node> = Vec::new();
    for (ia, &k) in b.iter().enumerate() {
        for &j in &b[ia + 1..] {
            let d = norm(&sub(&w.p.x[j], &w.p.x[k]));
            let aj = w.dv.a[j];
            let mgg = w.dv.m_g[k] * w.dv.m_g[j];
            let u = if d >= aj {
                -g_eff * mgg / d
            } else {
                -g_eff * mgg / aj - interior_potential_integral(w, k, j, d, &mut scratch)
            };
            s.add(u);
        }
    }
    s.value()
}

pub fn conservation(w: &World) -> Conservation {
    let (mut ke, mut mv) = (Neum::default(), Neum::default());
    let (mut pm, mut lm) = (Neum3::default(), Neum3::default());
    for k in body_indices(&w.p) {
        let (m, v) = (w.p.m[k], &w.p.v[k]);
        ke.add(0.5 * m * dot(v, v));
        pm.add_scaled(m, v);
        lm.add_scaled(m, &cross(&w.p.x[k], v));
        mv.add(m * norm(v));
    }
    let pe = potential_energy(w);
    Conservation {
        ke: ke.value(),
        pe,
        e: ke.value() + pe,
        p: pm.value(),
        l: lm.value(),
        sum_m_abs_v: mv.value(),
        label: energy_label(w),
    }
}

/// R_P = |sum F| / sum |F| over bodies: (shadow force only, total force). With V1/V2 drag the total
/// is physically non-zero (the flux absorbs momentum), so the shadow-only value is the quadrature check.
pub fn r_p_pair(w: &World) -> (f64, f64) {
    let (mut ss, mut ts) = (Neum3::default(), Neum3::default());
    let (mut sa, mut ta) = (Neum::default(), Neum::default());
    for k in body_indices(&w.p) {
        let fs = add(&w.force.f_pair[k], &w.force.f_corr[k]);
        ss.add(&fs);
        sa.add(norm(&fs));
        ts.add(&w.force.f_total[k]);
        ta.add(norm(&w.force.f_total[k]));
    }
    let r = |s: &Neum3, a: &Neum| {
        let d = a.value();
        if d > 0.0 {
            norm(&s.value()) / d
        } else {
            0.0
        }
    };
    (r(&ss, &sa), r(&ts, &ta))
}

#[derive(Clone, Debug, Default)]
pub struct Ledger {
    pub e0: f64,
    pub p0: V3,
    pub l0: V3,
    pub e_last: f64,
    pub de: f64,
    pub dp: f64,
    pub dl: f64,
    pub max_de: f64,
    pub max_dp: f64,
    pub max_dl: f64,
    pub rp_shadow: f64,
    pub rp_total: f64,
    pub max_rp_shadow: f64,
    pub max_rp_total: f64,
    pub samples: u64,
    pub label: EnergyLabel,
    pub ever_nonconservative: bool,
}
impl Ledger {
    pub fn start(c: &Conservation, rp_s: f64, rp_t: f64) -> Ledger {
        Ledger {
            e0: c.e,
            p0: c.p,
            l0: c.l,
            e_last: c.e,
            rp_shadow: rp_s,
            rp_total: rp_t,
            max_rp_shadow: rp_s,
            max_rp_total: rp_t,
            samples: 1,
            label: c.label,
            ever_nonconservative: c.label == EnergyLabel::DiagnosticNonConservative,
            ..Default::default()
        }
    }
    /// |dE|/|E0|, |dP|/sum m|v|, |dL|/|L0| (absolute if the denominator is zero), with running maxima.
    pub fn update(&mut self, c: &Conservation, rp_s: f64, rp_t: f64) {
        self.e_last = c.e;
        self.de = safe_ratio((c.e - self.e0).abs(), self.e0.abs());
        self.dp = safe_ratio(norm(&sub(&c.p, &self.p0)), c.sum_m_abs_v);
        self.dl = safe_ratio(norm(&sub(&c.l, &self.l0)), norm(&self.l0));
        self.max_de = self.max_de.max(self.de);
        self.max_dp = self.max_dp.max(self.dp);
        self.max_dl = self.max_dl.max(self.dl);
        self.rp_shadow = rp_s;
        self.rp_total = rp_t;
        self.max_rp_shadow = self.max_rp_shadow.max(rp_s);
        self.max_rp_total = self.max_rp_total.max(rp_t);
        self.samples += 1;
        self.label = c.label;
        if c.label == EnergyLabel::DiagnosticNonConservative {
            self.ever_nonconservative = true;
        }
    }
}

/// Spec section 6, item 9: equivalence-principle measure eta_EP = 2 |a_k - a_j| / (|a_k| + |a_j|) for two test bodies
/// (accelerations measured in the same field, e.g. from two otherwise identical runs differing in species offset delta).
pub fn eta_ep(a_k: &V3, a_j: &V3) -> f64 {
    let den = norm(a_k) + norm(a_j);
    if den > 0.0 {
        2.0 * norm(&sub(a_k, a_j)) / den
    } else {
        0.0
    }
}

// ------------------------------------------------------------------ per-target force diagnostics
/// Delta^NA_k = |F_sh - Gamma sum P chat| / |F_sh| = |F_corr| / |F_pair + F_corr|.
pub fn delta_na(w: &World, k: usize) -> f64 {
    let fsh = norm(&add(&w.force.f_pair[k], &w.force.f_corr[k]));
    if fsh > 0.0 {
        norm(&w.force.f_corr[k]) / fsh
    } else {
        0.0
    }
}

/// (uniform-sphere thin-field force, unsaturated point-mass Newtonian force) on body k.
pub fn thin_and_newton(w: &World, k: usize) -> (V3, V3) {
    let kap = |i: usize| w.g.kappa0 * (1.0 + w.p.delta[i]);
    let gam_thin = w.g.xi * w.g.pi_flux() * kap(k) * w.p.m[k] / (4.0 * PI);
    let (mut th, mut nw) = (Neum3::default(), Neum3::default());
    for j in body_indices(&w.p) {
        if j == k {
            continue;
        }
        let c = sub(&w.p.x[j], &w.p.x[k]);
        let d = norm(&c);
        let a = w.dv.a[j];
        if d < 1e-12 * a {
            continue;
        }
        let chat = scale(&c, 1.0 / d);
        let pth = if d > a {
            kap(j) * w.p.m[j] / (d * d)
        } else {
            kap(j) * w.p.m[j] * d / (a * a * a)
        };
        th.add_scaled(pth, &chat);
        nw.add_scaled((1.0 + w.p.delta[j]) * w.p.m[j] / (d * d), &chat);
    }
    (
        scale(&th.value(), gam_thin),
        scale(&nw.value(), w.g.g_n * w.p.m[k] * (1.0 + w.p.delta[k])),
    )
}

#[derive(Clone, Debug)]
pub struct SkyHeat {
    pub d_bar: f64,
    pub tau_bar: f64,
    pub p_heat: f64,
    pub p_heat_per_mass: f64,
}

/// D_bar (eq. 20), tau_bar and P_heat for body k from the instrumented sinks (None otherwise).
pub fn sky_and_heating(w: &World, k: usize) -> Option<SkyHeat> {
    let s = w.force.sinks.as_ref()?;
    let (mut om, mut tb, mut lf) = (Neum::default(), Neum::default(), Neum::default());
    for pm in &s.pairs {
        if pm.k as usize == k {
            om.add(pm.omega);
            tb.add(pm.tau_bar);
        }
    }
    for lr in &s.leaves {
        if lr.k as usize == k {
            let ns = w.g.nside0 << (lr.level as u32);
            lf.add(cell_area(ns) * (lr.d_total - lr.d_pair_sum));
        }
    }
    let d_bar = (om.value() + lf.value()) / (4.0 * PI);
    let p_heat = w.cfg.eta_e * w.cfg.zeta * w.g.pi_flux() * w.g.v_g * w.dv.a_eff[k] * (1.0 - d_bar);
    Some(SkyHeat {
        d_bar,
        tau_bar: tb.value(),
        p_heat,
        p_heat_per_mass: p_heat / w.p.m[k],
    })
}

/// Isolated-thin reference P_heat/m = 4 pi G_N eta_E zeta v_g / (xi kappa0).
pub fn heat_reference_per_mass(w: &World) -> f64 {
    4.0 * PI * w.g.g_n * w.cfg.eta_e * w.cfg.zeta * w.g.v_g / (w.g.xi * w.g.kappa0)
}

/// gamma_k = |F_dr| / (m |v - W|); None in V0 or at rest in the flux frame.
pub fn drag_rate(w: &World, k: usize) -> Option<f64> {
    if w.opt.mode == Mode::V0 {
        return None;
    }
    let u = norm(&sub(&w.p.v[k], &w.g.w));
    if u > 0.0 {
        Some(norm(&w.force.f_dr[k]) / (w.p.m[k] * u))
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub struct TargetDiag {
    pub k: usize,
    pub delta_na: f64,
    pub delta_sat: f64,
    pub r_newton: f64,
    pub r_thin: f64,
    pub gamma_drag: Option<f64>,
    pub sky_heat: Option<SkyHeat>,
}

pub fn target_diagnostics(w: &World) -> Vec<TargetDiag> {
    body_indices(&w.p)
        .into_iter()
        .map(|k| {
            let (thin, newt) = thin_and_newton(w, k);
            TargetDiag {
                k,
                delta_na: delta_na(w, k),
                delta_sat: rel_resid(&w.force.f_pair[k], &thin), // Delta^sat
                r_newton: rel_resid(&w.force.f_total[k], &newt), // R^N (F_k = total)
                r_thin: rel_resid(&w.force.f_total[k], &thin),   // R^thin
                gamma_drag: drag_rate(w, k),
                sky_heat: sky_and_heating(w, k),
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct PairG {
    pub k: usize,
    pub j: usize,
    pub g_eff: f64,
    pub expected: Option<f64>,
}

/// G_eff,kj = |Gamma_k P_kj| d^2 / (m_k m_j) from the sink P_kj; expected G (1+d_k)(1+d_j) S_k S_j
/// for d > a_j and alpha_bg = 0. Needs instrumented evaluation.
pub fn pair_effective_g(w: &World) -> Option<Vec<PairG>> {
    let s = w.force.sinks.as_ref()?;
    let pi_flux = w.g.pi_flux();
    let g_const = w.g.xi * pi_flux * w.g.kappa0 * w.g.kappa0 / (4.0 * PI);
    let mut out = Vec::new();
    for pm in &s.pairs {
        let (k, j) = (pm.k as usize, pm.j as usize);
        if w.p.kind[k] != Kind::Body {
            continue;
        }
        let d = norm(&sub(&w.p.x[j], &w.p.x[k]));
        let gamma_k = w.g.xi * pi_flux * w.dv.a_eff[k] / (4.0 * PI);
        let g_eff = (gamma_k * pm.p).abs() * d * d / (w.p.m[k] * w.p.m[j]);
        let expected = if d > w.dv.a[j] && w.g.alpha_bg == 0.0 {
            Some(g_const * (1.0 + w.p.delta[k]) * (1.0 + w.p.delta[j]) * w.dv.s[k] * w.dv.s[j])
        } else {
            None
        };
        out.push(PairG {
            k,
            j,
            g_eff,
            expected,
        });
    }
    Some(out)
}

#[derive(Clone, Debug)]
pub struct ProbeG {
    pub index: usize,
    pub r: f64,
    pub g_eff: f64,
}

/// Probes: G_eff(r) = |g_p| r^2 / sum m^g, r from the m^g-weighted centroid of the bodies.
pub fn probe_effective_g(w: &World) -> Vec<ProbeG> {
    let (mut mg, mut c) = (Neum::default(), Neum3::default());
    for k in body_indices(&w.p) {
        mg.add(w.dv.m_g[k]);
        c.add_scaled(w.dv.m_g[k], &w.p.x[k]);
    }
    let m = mg.value();
    if crate::mathx::not_gt(m, 0.0) {
        return Vec::new();
    }
    let cen = scale(&c.value(), 1.0 / m);
    (0..w.p.x.len())
        .filter(|&i| w.p.kind[i] == Kind::Probe)
        .map(|i| {
            let r = norm(&sub(&w.p.x[i], &cen));
            ProbeG {
                index: i,
                r,
                g_eff: norm(&w.force.f_total[i]) * r * r / m,
            }
        })
        .collect()
}

// ------------------------------------------------------------------ Spec 6.11: brute-force comparison (reference mode)
/// Default grid of the live reference diagnostic (n_chi x n_phi samples per occluder disc; `reference_forces` also runs 2x).
pub const REF_DIAG_N_CHI: usize = 100;
pub const REF_DIAG_N_PHI: usize = 200;

/// Engine-versus-reference error (Spec 6 item 11) in a form the front end can carry. Opt-in: it costs a full all-rays evaluation.
#[derive(Clone, Debug)]
pub struct ReferenceDiag {
    pub n_chi: usize,
    pub n_phi: usize,
    /// Ok: the comparison. Err: why the world is outside reference scope (V0, alpha_bg = 0, bodies only, exterior pairs, N <= 50)
    /// or why the grid is unusable; in that case the engine is simply NOT checked, which is not the same as agreeing.
    pub outcome: Result<ReferenceCheck, String>,
}
impl ReferenceDiag {
    /// max over bodies of |F_engine - F_reference| / max |F_reference| (None when out of scope).
    pub fn max_rel_diff(&self) -> Option<f64> {
        self.outcome.as_ref().ok().map(|c| c.max_rel_diff)
    }
    /// The reference's own resolution error (differences below this are not meaningful).
    pub fn reference_est_err(&self) -> Option<f64> {
        self.outcome.as_ref().ok().map(|c| c.reference_est_err)
    }
    /// True when the engine-reference difference is larger than the reference's own resolution estimate, i.e. it is not
    /// explained by the reference being coarse. None when out of scope.
    pub fn exceeds_reference_resolution(&self) -> Option<bool> {
        self.outcome
            .as_ref()
            .ok()
            .map(|c| c.max_rel_diff > c.reference_est_err)
    }
    /// One-line text for a read-out row (the UI shows it as an "extras" key/value pair, no egui change needed).
    pub fn summary(&self) -> String {
        match &self.outcome {
            Ok(c) => format!("max |engine - reference| / |F| = {:.2e}; the reference's own quadrature error is about {:.1e}: {}",
                             c.max_rel_diff, c.reference_est_err,
                             if c.max_rel_diff > c.reference_est_err {
                                 "the gap exceeds it, so a coarse reference does not explain it"
                             } else {
                                 "the gap is within it, so it cannot be told apart from reference error"
                             }),
            Err(why) => format!("not checked: {why}"),
        }
    }
}
fn core_error_text(e: &CoreError) -> String {
    match e {
        CoreError::Config(m) | CoreError::Gating(m) => (*m).to_string(),
        other => format!("{other:?}"),
    }
}
/// Compare the engine forces currently stored in `w` with the all-rays reference (Spec 4.4 / 6 item 11). Never panics: an
/// out-of-scope world yields `outcome = Err(reason)`.
pub fn reference_diag(w: &World, n_chi: usize, n_phi: usize) -> ReferenceDiag {
    if n_chi < 2 || n_phi < 4 {
        return ReferenceDiag {
            n_chi,
            n_phi,
            outcome: Err("reference grid too small (need n_chi >= 2, n_phi >= 4)".to_string()),
        };
    }
    let outcome = reference_check(w, n_chi, n_phi).map_err(|e| core_error_text(&e));
    ReferenceDiag {
        n_chi,
        n_phi,
        outcome,
    }
}

// ------------------------------------------------------------------ Spec 6.6: tau_max
/// Additive optical depth along direction `n` (unit) seen from body k: tau_k(n) = sum_{j != k} beta_j L_j(n), L_j the chord of
/// sphere j along the ray (eq. 16; interior target: exit distance; coincident centres: L = a, as in Spec 1.6). Material only:
/// the Seeliger term alpha_bg is not part of an additive optical depth. Static geometry (see `TauMax::static_geometry_only`).
pub fn tau_along(w: &World, k: usize, n: &V3) -> f64 {
    tau_along_in(w, k, n, &body_indices(&w.p))
}

fn tau_along_in(w: &World, k: usize, n: &V3, bodies: &[usize]) -> f64 {
    let mut s = Neum::default();
    for &j in bodies {
        if j == k {
            continue;
        }
        let c = sub(&w.p.x[j], &w.p.x[k]);
        if let Some((_, _, l)) = chord(&c, w.dv.a[j], n) {
            s.add(w.dv.beta[j] * l);
        }
    }
    s.value()
}

/// Spec 6 item 6: a LOWER BOUND on the maximum over the sky of the additive optical depth seen by body k. The true maximum is
/// not computed; the bound is the largest tau_k(n) over the sample set {c_kj} (directions to the other bodies' centres, always
/// available) united with {n_c} (centres of the quadrature leaves, only when `Options.instrumented` recorded them).
/// Exact checks: an exterior single occluder gives tau_c of that occluder; collinear nested occluders add.
#[derive(Clone, Debug)]
pub struct TauMax {
    pub k: usize,
    /// max of `from_centres` and `from_leaves`.
    pub lower_bound: f64,
    pub from_centres: f64,
    /// None unless the world was evaluated with `instrumented = true` (leaf records present). Some(0.0) when instrumented
    /// but the target has no overlap leaves.
    pub from_leaves: Option<f64>,
    /// The sampled direction that attains `lower_bound`.
    pub direction: V3,
    pub samples: u64,
    /// True in V2: the sphere-wise shifted geometry (eq. 9) is NOT used, so the value refers to the static sky.
    pub static_geometry_only: bool,
}

/// tau_max lower bound for every body. Cost: O(N^2 * N) for the centre set plus O(leaves * N) when leaf records exist.
pub fn tau_max(w: &World) -> Vec<TauMax> {
    let bodies = body_indices(&w.p);
    let has_leaves = w.force.sinks.is_some();
    let mut slot = vec![usize::MAX; w.p.x.len()];
    let mut out: Vec<TauMax> = Vec::with_capacity(bodies.len());
    for (s, &k) in bodies.iter().enumerate() {
        slot[k] = s;
        let mut best: (f64, V3) = (0.0, [0.0, 0.0, 1.0]);
        let mut samples = 0u64;
        for &j in &bodies {
            if j == k {
                continue;
            }
            let c = sub(&w.p.x[j], &w.p.x[k]);
            let d = norm(&c);
            let n: V3 = if d < 1e-12 * w.dv.a[j] {
                [0.0, 0.0, 1.0]
            } else {
                scale(&c, 1.0 / d)
            };
            let t = tau_along_in(w, k, &n, &bodies);
            samples += 1;
            if t > best.0 {
                best = (t, n);
            }
        }
        out.push(TauMax {
            k,
            lower_bound: best.0,
            from_centres: best.0,
            from_leaves: if has_leaves { Some(0.0) } else { None },
            direction: best.1,
            samples,
            static_geometry_only: w.opt.mode == Mode::V2,
        });
    }
    if let Some(sinks) = w.force.sinks.as_ref() {
        for lr in &sinks.leaves {
            let sl = slot[lr.k as usize];
            if sl == usize::MAX {
                continue;
            }
            let n = pix2vec_nest(w.g.nside0 << (lr.level as u32), lr.pix);
            let t = tau_along_in(w, lr.k as usize, &n, &bodies);
            let o = &mut out[sl];
            o.samples += 1;
            if t > o.from_leaves.unwrap_or(0.0) {
                o.from_leaves = Some(t);
            }
            if t > o.lower_bound {
                o.lower_bound = t;
                o.direction = n;
            }
        }
    }
    out
}

// ------------------------------------------------------------------ Spec 6.8: Seeliger paired read-out
/// One ordered pair (target k, occluder j): P_kj(alpha) / P_kj(0) against exp(-alpha d). F_kj = Gamma_k P_kj chat, so this is
/// the pair-force ratio F(alpha)/F(0).
#[derive(Clone, Debug)]
pub struct SeeligerPair {
    pub k: usize,
    pub j: usize,
    pub d: f64,
    pub ratio: f64,
    /// exp(-alpha d); None for an interior pair (d <= a_j), where the law is not the comparison the Spec makes.
    pub expected: Option<f64>,
    /// |ratio / expected - 1|. Agreement with exp(-alpha d) is only expected for a thin occluder far from the target (the ray's
    /// near-entry distance is not d, and the occluder has optical depth); no pass/fail threshold is applied here. Reference
    /// value [P, scipy]: 1.0e-6 for tau_c = 1e-6, d = 100 a, alpha d = 0.05 (the geometry of tests/diag_gaps.rs).
    pub rel_dev: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct SeeligerCheck {
    pub alpha: f64,
    pub pairs: Vec<SeeligerPair>,
    /// Per body: |F_sh(alpha)| / |F_sh(0)| (F_sh = F_pair + F_corr) from two FULL force evaluations of the same state. Equals the
    /// pair ratio only when a single pair dominates and nothing overlaps; with several occluders it is a raw read-out.
    pub body_ratio: Vec<(usize, f64)>,
    pub max_pair_dev: Option<f64>,
}

/// Paired-run Seeliger diagnostic at the current state: evaluates the force with alpha_bg = 0 and with alpha_bg = `alpha`
/// (everything else identical, `w` untouched) and reads out F(alpha)/F(0) per pair and per body. Costs two full force
/// evaluations. V2 is refused (the Spec gates alpha_bg > 0 out of V2); `thin_fast` is switched off for both runs.
pub fn seeliger_check(w: &World, alpha: f64) -> Result<SeeligerCheck, CoreError> {
    if crate::mathx::not_gt(alpha, 0.0) || !alpha.is_finite() {
        return Err(CoreError::Config(
            "seeliger check: alpha must be finite and > 0",
        ));
    }
    if w.opt.mode == Mode::V2 {
        return Err(CoreError::Gating(
            "seeliger check: V2 requires alpha_bg = 0",
        ));
    }
    let bodies = body_indices(&w.p);
    let mut scratch: Vec<Node> = Vec::new();
    let mut pairs: Vec<SeeligerPair> = Vec::new();
    for &k in &bodies {
        for &j in &bodies {
            if j == k {
                continue;
            }
            let d = norm(&sub(&w.p.x[j], &w.p.x[k]));
            let (a, beta, a_eff) = (w.dv.a[j], w.dv.beta[j], w.dv.a_eff[j]);
            let p0 = pair_p(
                PairSpec {
                    d,
                    a,
                    beta,
                    alpha: 0.0,
                    a_eff,
                    force_gl: w.opt.force_gl,
                },
                &w.quad,
                &mut scratch,
            );
            if crate::mathx::not_gt(p0, 0.0) {
                continue;
            } // coincident centres / lambda = 0: nothing to compare
            let pa = pair_p(
                PairSpec {
                    d,
                    a,
                    beta,
                    alpha,
                    a_eff,
                    force_gl: w.opt.force_gl,
                },
                &w.quad,
                &mut scratch,
            );
            let ratio = pa / p0;
            let expected = if d > a {
                Some(mathx::exp(-alpha * d))
            } else {
                None
            };
            let rel_dev = expected.map(|e| (ratio / e - 1.0).abs());
            pairs.push(SeeligerPair {
                k,
                j,
                d,
                ratio,
                expected,
                rel_dev,
            });
        }
    }
    let mut opt = w.opt.clone();
    opt.instrumented = false;
    opt.thin_fast = false;
    let mut g0 = w.g.clone();
    g0.alpha_bg = 0.0;
    let mut ga = w.g.clone();
    ga.alpha_bg = alpha;
    let f0 = crate::evaluate_forces(&w.p, &w.dv, &g0, &opt)?;
    let fa = crate::evaluate_forces(&w.p, &w.dv, &ga, &opt)?;
    let mut body_ratio: Vec<(usize, f64)> = Vec::new();
    for &k in &bodies {
        let s0 = norm(&add(&f0.f_pair[k], &f0.f_corr[k]));
        let sa = norm(&add(&fa.f_pair[k], &fa.f_corr[k]));
        if s0 > 0.0 {
            body_ratio.push((k, sa / s0));
        }
    }
    let max_pair_dev = pairs
        .iter()
        .filter_map(|p| p.rel_dev)
        .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |y| y.max(x))));
    Ok(SeeligerCheck {
        alpha,
        pairs,
        body_ratio,
        max_pair_dev,
    })
}

// ------------------------------------------------------------------ snapshot
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub time: f64,
    pub step: u64,
    pub conservation: Conservation,
    pub ledger: Ledger,
    pub targets: Vec<TargetDiag>,
    pub status: Status,
    pub labels: Vec<&'static str>,
    /// Only when requested through `SnapshotOptions::reference` (None otherwise: NOT "checked and fine").
    pub reference: Option<ReferenceDiag>,
    /// Only when requested through `SnapshotOptions::tau_max`.
    pub tau_max: Option<Vec<TauMax>>,
}
/// Opt-in extras for `snapshot_with`. Default: nothing extra, i.e. exactly the old `snapshot`.
#[derive(Clone, Debug, Default)]
pub struct SnapshotOptions {
    /// Some((n_chi, n_phi)): also run `reference_diag` (all-rays evaluation: seconds at larger N, N <= 50 only).
    pub reference: Option<(usize, usize)>,
    /// Also compute the tau_max lower bounds (`tau_max`).
    pub tau_max: bool,
}
/// Everything above in one immutable record (no force evaluation).
pub fn snapshot(w: &World) -> Snapshot {
    snapshot_with(w, &SnapshotOptions::default())
}

/// `snapshot` plus the opt-in diagnostics of `opts`. Still no engine force evaluation (the reference mode runs its own all-rays
/// integral, and does not alter `w`).
pub fn snapshot_with(w: &World, opts: &SnapshotOptions) -> Snapshot {
    Snapshot {
        time: w.time(),
        step: w.step_count,
        conservation: conservation(w),
        ledger: w.ledger.clone(),
        targets: target_diagnostics(w),
        status: w.force.status.clone(),
        labels: w.labels(),
        reference: opts.reference.map(|(c, p)| reference_diag(w, c, p)),
        tau_max: if opts.tau_max { Some(tau_max(w)) } else { None },
    }
}

// ------------------------------------------------------------------------------------------------ Spec 6 aggregates
/// min / mean / max of a per-body quantity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range3 {
    pub min: f64,
    pub mean: f64,
    pub max: f64,
}
fn range3(v: &[f64]) -> Option<Range3> {
    if v.is_empty() {
        return None;
    }
    let mut s = Neum::default();
    for &x in v {
        s.add(x);
    }
    Some(Range3 {
        min: v.iter().cloned().fold(f64::INFINITY, f64::min),
        mean: s.value() / v.len() as f64,
        max: v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    })
}
/// Spec 6 items 6, 7 and 4 ("report ... min/mean/max", "mean and max of D_bar", "total power"). The D_bar and power entries exist
/// only for an instrumented world (None otherwise: not measured, which is not the same as zero).
#[derive(Clone, Debug)]
pub struct Aggregates {
    pub tau_c: Option<Range3>,
    pub s: Option<Range3>,
    pub d_bar: Option<Range3>,
    pub p_heat_total: Option<f64>,
}
pub fn aggregates(w: &World) -> Aggregates {
    let b = body_indices(&w.p);
    let tau: Vec<f64> = b.iter().map(|&k| w.dv.tau_c[k]).collect();
    let s: Vec<f64> = b.iter().map(|&k| w.dv.s[k]).collect();
    let heat: Vec<SkyHeat> = b.iter().filter_map(|&k| sky_and_heating(w, k)).collect();
    let measured = !b.is_empty() && heat.len() == b.len();
    let d: Vec<f64> = heat.iter().map(|h| h.d_bar).collect();
    let mut tot = Neum::default();
    for h in &heat {
        tot.add(h.p_heat);
    }
    Aggregates {
        tau_c: range3(&tau),
        s: range3(&s),
        d_bar: if measured { range3(&d) } else { None },
        p_heat_total: if measured { Some(tot.value()) } else { None },
    }
}

/// Spec parameter-table ranges (Spec 6, parameter tables). The engine accepts a wider domain (verification presets need it), so a
/// value outside the table is a WARNING the caller should show, never a silent pass.
pub fn spec_range_warnings(w: &World) -> Vec<String> {
    let g = &w.g;
    let mut v = Vec::new();
    let mut chk = |name: &str, x: f64, lo: f64, hi: f64| {
        if !(lo..=hi).contains(&x) {
            v.push(format!(
                "{name} = {x:e} is outside the Spec range {lo:e} to {hi:e}"
            ));
        }
    };
    chk("kappa0", g.kappa0, 1e-12, 1e-2);
    chk("xi", g.xi, 0.5, 2.0);
    chk("v_g", g.v_g, 10.0, 1e8);
    chk("f", g.f, 0.05, 0.5);
    for (k, &d) in w.p.delta.iter().enumerate() {
        if w.p.kind[k] == Kind::Body && !(0.0..=1e-3).contains(&d) {
            v.push(format!(
                "delta[{k}] = {d:e} is outside the Spec range 0 to 1e-3"
            ));
        }
    }
    v
}
