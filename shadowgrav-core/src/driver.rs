//! Minimal driver: run helpers and the standard test problems (a)-(d) at integrator level.
//! No I/O, no rendering. Everything returns plain report structs for a future harness.

use crate::diagnostics::{self, EnergyLabel};
use crate::geom::*;
use crate::integrator::advance;
use crate::state::*;
use crate::sum::Neum;
use crate::types::*;
use std::f64::consts::PI;

/// Spec table default: a = 0.01 at m = 1.
pub const RHO0: f64 = 2.39e5;

pub fn sphere_radius(m: f64, rho: f64) -> f64 {
    crate::mathx::cbrt(3.0 * m / (4.0 * PI * rho))
}
/// tau_c = 3 kappa m / (2 pi a^2)  =>  kappa0 = 2 pi a^2 tau_c / (3 m).
pub fn kappa0_for_tau(tau_c: f64, m: f64, rho: f64) -> f64 {
    let a = sphere_radius(m, rho);
    2.0 * PI * a * a * tau_c / (3.0 * m)
}
pub fn default_options(mode: Mode) -> Options {
    Options {
        mode,
        instrumented: false,
        force_gl: false,
        leaf_budget: 0,
        validation: None,
        parallel: true,
        brute_candidates: false,
        thin_fast: false,
        thin_theta: 0.5,
    }
}
pub fn body_particles(xs: Vec<V3>, vs: Vec<V3>) -> Particles {
    let n = xs.len();
    Particles {
        x: xs,
        v: vs,
        m: vec![1.0; n],
        rho: vec![RHO0; n],
        delta: vec![0.0; n],
        kind: vec![Kind::Body; n],
    }
}

/// Two-sided centred least squares; returns (slope, intercept).
pub fn linfit(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let (mut sx, mut sy) = (Neum::default(), Neum::default());
    for i in 0..x.len() {
        sx.add(x[i]);
        sy.add(y[i]);
    }
    let (mx, my) = (sx.value() / n, sy.value() / n);
    let (mut sxx, mut sxy) = (Neum::default(), Neum::default());
    for i in 0..x.len() {
        let dx = x[i] - mx;
        sxx.add(dx * dx);
        sxy.add(dx * (y[i] - my));
    }
    let slope = sxy.value() / sxx.value();
    (slope, my - slope * mx)
}

pub fn run_steps<F: FnMut(&World)>(w: &mut World, n: u64, mut observe: F) -> Result<(), SimError> {
    for _ in 0..n {
        advance(w)?;
        observe(w);
    }
    Ok(())
}
pub fn run_until<F: FnMut(&World)>(
    w: &mut World,
    t_end: f64,
    mut observe: F,
) -> Result<u64, SimError> {
    let mut n = 0;
    while w.time() < t_end {
        advance(w)?;
        observe(w);
        n += 1;
    }
    Ok(n)
}

// ======================================================================= test (a): two-body orbit
pub struct OrbitSetup {
    pub world: World,
    pub omega: f64,
    pub period: f64,
    pub h: f64,
    pub steps_per_orbit: u32,
}

/// m1 = m2 = 1, a ~ 0.01, r0 = 1, centre-of-mass frame. Relative speed v = sqrt(r0 F / mu) with F the
/// model's own radial force (Spec 7a), mu = 1/2. h = T/steps_per_orbit, fixed.
pub fn two_body_orbit(
    tau_c: f64,
    steps_per_orbit: u32,
    mode: Mode,
    v_g: f64,
) -> Result<OrbitSetup, SimError> {
    let r0 = 1.0;
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau_c, 1.0, RHO0);
    g.v_g = v_g;
    let p = body_particles(
        vec![[-0.5 * r0, 0.0, 0.0], [0.5 * r0, 0.0, 0.0]],
        vec![[0.0; 3]; 2],
    );
    let mut w = World::new(p, g, default_options(mode), SimConfig::fixed(1.0))?;
    let f = norm(&w.force.f_total[0]);
    let mu = 0.5;
    let v_rel = (r0 * f / mu).sqrt();
    let omega = v_rel / r0;
    let period = 2.0 * PI / omega;
    let h = period / steps_per_orbit as f64;
    w.set_step_control(StepControl::Fixed { h })?;
    w.set_velocities(vec![[0.0, -0.5 * v_rel, 0.0], [0.0, 0.5 * v_rel, 0.0]])?;
    Ok(OrbitSetup {
        world: w,
        omega,
        period,
        h,
        steps_per_orbit,
    })
}

#[derive(Debug)]
pub struct OrbitReport {
    pub steps: u64,
    pub omega_expected: f64,
    pub omega_h: f64,
    pub period_expected: f64,
    pub period_measured: f64,
    pub period_rel_err: f64,
    pub max_de: f64,
    pub secular_de: f64,
    pub max_dl: f64,
    pub max_dp: f64,
    pub max_rp_total: f64,
    pub max_rp_shadow: f64,
    pub max_delta_na: f64,
    pub max_fp_iters: u32,
    pub fp_nonconverged: u64,
    pub label: EnergyLabel,
}

fn rel_vec(w: &World) -> V3 {
    sub(&w.p.x[1], &w.p.x[0])
}

/// Run n_orbits, fit phi(t) = phi0 + omega t for the period, and collect the ledger.
pub fn run_orbit(mut s: OrbitSetup, n_orbits: u32) -> Result<OrbitReport, SimError> {
    let steps = s.steps_per_orbit as u64 * n_orbits as u64;
    let (omega, period, h) = (s.omega, s.period, s.h);
    let w = &mut s.world;
    let cap = steps as usize + 1;
    let (mut ts, mut phis, mut es) = (
        Vec::with_capacity(cap),
        Vec::with_capacity(cap),
        Vec::with_capacity(cap),
    );
    let r = rel_vec(w);
    let mut prev = crate::mathx::atan2(r[1], r[0]);
    let mut unwrapped = prev;
    ts.push(w.time());
    phis.push(unwrapped);
    es.push(w.ledger.e_last);
    let (mut max_dna, mut max_iters) = (0.0f64, 0u32);
    for _ in 0..steps {
        let st = advance(w)?;
        max_iters = max_iters.max(st.fp_iters);
        let r = rel_vec(w);
        let cur = crate::mathx::atan2(r[1], r[0]);
        let mut d = cur - prev;
        if d > PI {
            d -= 2.0 * PI
        } else if d < -PI {
            d += 2.0 * PI
        }
        unwrapped += d;
        prev = cur;
        ts.push(w.time());
        phis.push(unwrapped);
        es.push(w.ledger.e_last);
        for k in 0..2 {
            max_dna = max_dna.max(diagnostics::delta_na(w, k));
        }
    }
    let (om_fit, _) = linfit(&ts, &phis);
    let (e_slope, _) = linfit(&ts, &es);
    let t_span = ts[ts.len() - 1] - ts[0];
    let period_measured = 2.0 * PI / om_fit;
    Ok(OrbitReport {
        steps,
        omega_expected: omega,
        omega_h: omega * h,
        period_expected: period,
        period_measured,
        period_rel_err: (period_measured - period).abs() / period,
        max_de: w.ledger.max_de,
        secular_de: (e_slope * t_span).abs() / w.ledger.e0.abs(),
        max_dl: w.ledger.max_dl,
        max_dp: w.ledger.max_dp,
        max_rp_total: w.ledger.max_rp_total,
        max_rp_shadow: w.ledger.max_rp_shadow,
        max_delta_na: max_dna,
        max_fp_iters: max_iters,
        fp_nonconverged: w.flags.fp_nonconverged_steps,
        label: w.ledger.label,
    })
}

/// One acceptance row. `affects_process_exit` is always false here: these rows report frozen-PDF text and the
/// proposed Addendum 2 bound side by side. They do not select which bound gates the process.
#[derive(Clone, Debug)]
pub struct OrbitCriterion {
    pub name: &'static str,
    pub provenance: &'static str,
    pub measured: f64,
    pub bound: f64,
    pub pass: bool,
}

/// Frozen PDF (1 Nov 2026, section 7 test (a)) and proposed Addendum 2 item 1, evaluated on the same `OrbitReport`.
/// The period quantity is `|T_measured - T_expected| / T_expected` from `run_orbit` (mean-motion fit). It is not redefined here.
pub fn orbit_criteria(r: &OrbitReport) -> Vec<OrbitCriterion> {
    let wh2 = r.omega_h * r.omega_h;
    vec![
        OrbitCriterion {
            name: "PDF 7(a) |T_measured-T|/T <= 5e-6",
            provenance: "frozen PDF section 7 test (a); not Addendum 2",
            measured: r.period_rel_err,
            bound: 5e-6,
            pass: r.period_rel_err <= 5e-6,
        },
        OrbitCriterion {
            name: "Addendum2 |T_measured-T|/T <= 0.36*(omega*h)^2",
            provenance: "ADDENDUM2.md item 1, proposed for the contract, not the frozen PDF",
            measured: r.period_rel_err,
            bound: 0.36 * wh2,
            pass: r.period_rel_err <= 0.36 * wh2,
        },
        OrbitCriterion {
            name: "PDF 7(a) max |dE/E| <= (omega*h)^2",
            provenance: "frozen PDF section 7 test (a)",
            measured: r.max_de,
            bound: wh2,
            pass: r.max_de <= wh2,
        },
        OrbitCriterion {
            name: "PDF 7(a) |dL/L| <= 1e-12",
            provenance: "frozen PDF section 7 test (a)",
            measured: r.max_dl,
            bound: 1e-12,
            pass: r.max_dl <= 1e-12,
        },
        OrbitCriterion {
            name: "PDF 7(a) R_P <= 1e-12",
            provenance: "frozen PDF section 7 test (a)",
            measured: r.max_rp_total,
            bound: 1e-12,
            pass: r.max_rp_total <= 1e-12,
        },
        OrbitCriterion {
            name: "PDF 7(a) Delta^NA == 0",
            provenance: "frozen PDF section 7 test (a)",
            measured: r.max_delta_na,
            bound: 0.0,
            pass: r.max_delta_na == 0.0,
        },
    ]
}

pub fn print_orbit_criteria(tag: &str, steps_per_orbit: u32, n_orbits: u32, r: &OrbitReport) {
    println!("--- independent criteria {tag} (printed, not a process-exit policy)");
    println!(
        "measurement=run_orbit mean-motion |2*pi/omega_fit - T|/T  h={:.16e}  steps_per_orbit={steps_per_orbit}  n_orbits={n_orbits}  steps={}  omega={:.16e}  omega*h={:.16e}  T_expected={:.16e}  T_measured={:.16e}  period_rel_err={:.16e}",
        r.omega_h / r.omega_expected,
        r.steps,
        r.omega_expected,
        r.omega_h,
        r.period_expected,
        r.period_measured,
        r.period_rel_err
    );
    for c in orbit_criteria(r) {
        println!(
            "[{}] {:<62} measured={:.6e}  bound={:.6e}  provenance: {}",
            if c.pass { "PASS" } else { "FAIL" },
            c.name,
            c.measured,
            c.bound,
            c.provenance
        );
    }
    println!(
        "[UNSCORED] secular |e_slope*t_span/E0|={:.6e}  PDF 7(a) says \"no secular growth\" and does not give a number; this line is not a pass/fail",
        r.secular_de
    );
    let rows = orbit_criteria(r);
    let pdf = rows[0].pass;
    let add = rows[1].pass;
    println!(
        "period_criteria_disagree={}  process_exit_adopts_neither_period_policy=true",
        pdf != add
    );
}

/// Test (d3) at integrator level: V1 and V2 orbits from identical initial data with large v_g
/// (|eps| ~ 1e-8). Returns the max relative-separation difference over one orbit, in units of r0.
/// (V2 = V1 + F_ab, so V1 is the comparison; V0 has no drag and is not comparable.)
pub fn run_v2_vs_v1(tau_c: f64, v_g: f64, steps_per_orbit: u32) -> Result<f64, SimError> {
    let mut a = two_body_orbit(tau_c, steps_per_orbit, Mode::V1, v_g)?;
    let mut b = two_body_orbit(tau_c, steps_per_orbit, Mode::V2, v_g)?;
    let mut worst = 0.0f64;
    for _ in 0..steps_per_orbit {
        advance(&mut a.world)?;
        advance(&mut b.world)?;
        worst = worst.max(norm(&sub(&rel_vec(&a.world), &rel_vec(&b.world))));
    }
    if b.world.flags.fp_nonconverged_steps > 0 {
        return Err(SimError::BadConfig("V2 fixed point did not converge"));
    }
    Ok(worst)
}

// ======================================================================= tests (d1), (d2): drag
pub fn drag_gamma_ref(g: &Globals) -> Result<f64, SimError> {
    let p = body_particles(vec![[0.0; 3]], vec![[0.0; 3]]);
    let dv = crate::derive::derive(&p, g)?;
    Ok(16.0 * PI * g.g_n * dv.s[0] * (1.0 + p.delta[0]) / (3.0 * g.kappa0 * g.v_g))
}
pub fn drag_world(g: Globals, v0: V3, h: f64) -> Result<World, SimError> {
    World::new(
        body_particles(vec![[0.0; 3]], vec![v0]),
        g,
        default_options(Mode::V1),
        SimConfig::fixed(h),
    )
}

#[derive(Debug)]
pub struct DragReport {
    pub gamma_ref: f64,
    pub gamma_fit: f64,
    pub rel_err: f64,
    pub h_gamma: f64,
    pub steps: u64,
    pub max_fp_iters: u32,
    pub nonconverged: u64,
}
/// Single thin body, W = 0, v_g / V0 = 1e3. Fit ln|v| vs t. NOTE: the scheme is trapezoidal in v, so the
/// fitted rate carries a relative bias (h gamma)^2 / 12; (d1)'s 1e-5 needs h gamma <= ~5e-3.
pub fn run_drag_decay(h_gamma: f64, n_decay_times: f64) -> Result<DragReport, SimError> {
    let g = Globals::defaults();
    let gamma_ref = drag_gamma_ref(&g)?;
    let v0 = g.v_g / 1e3;
    let h = h_gamma / gamma_ref;
    let mut w = drag_world(g, [v0, 0.0, 0.0], h)?;
    let steps = (n_decay_times / h_gamma).ceil() as u64;
    let (mut ts, mut ys) = (vec![w.time()], vec![crate::mathx::ln(w.p.v[0][0])]);
    let mut max_iters = 0;
    for _ in 0..steps {
        let st = advance(&mut w)?;
        max_iters = max_iters.max(st.fp_iters);
        ts.push(w.time());
        ys.push(crate::mathx::ln(w.p.v[0][0]));
    }
    let (slope, _) = linfit(&ts, &ys);
    let gamma_fit = -slope;
    Ok(DragReport {
        gamma_ref,
        gamma_fit,
        rel_err: (gamma_fit - gamma_ref).abs() / gamma_ref,
        h_gamma,
        steps,
        max_fp_iters: max_iters,
        nonconverged: w.flags.fp_nonconverged_steps,
    })
}

/// (d2): body moving at V0 with W = 0 versus body at rest with W = -V0. Returns the max relative
/// difference of u = v - W over the run (Galilean invariance of the flux frame).
pub fn run_frame_invariance(h_gamma: f64, n_decay_times: f64) -> Result<f64, SimError> {
    let g0 = Globals::defaults();
    let gamma_ref = drag_gamma_ref(&g0)?;
    let v0 = g0.v_g / 1e3;
    let h = h_gamma / gamma_ref;
    let mut ga = g0.clone();
    ga.w = [0.0; 3];
    let mut gb = g0.clone();
    gb.w = [-v0, 0.0, 0.0];
    let mut a = drag_world(ga, [v0, 0.0, 0.0], h)?;
    let mut b = drag_world(gb, [0.0; 3], h)?;
    let steps = (n_decay_times / h_gamma).ceil() as u64;
    let mut worst = 0.0f64;
    for _ in 0..steps {
        advance(&mut a)?;
        advance(&mut b)?;
        let ua = a.p.v[0];
        let ub = sub(&b.p.v[0], &b.g.w);
        worst = worst.max(norm(&sub(&ua, &ub)) / norm(&ua));
    }
    Ok(worst)
}

// ======================================================================= test (c) scenario
/// Three equal collinear spheres A, B, C with d_AB = d_BC = 10 a, at rest.
pub fn three_body_collinear(tau_c: f64, h: f64, leaf_budget: u64) -> Result<World, SimError> {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau_c, 1.0, RHO0);
    let p = body_particles(
        vec![[0.0; 3], [10.0 * a, 0.0, 0.0], [20.0 * a, 0.0, 0.0]],
        vec![[0.0; 3]; 3],
    );
    let mut opt = default_options(Mode::V0);
    opt.leaf_budget = leaf_budget;
    World::new(p, g, opt, SimConfig::fixed(h))
}
/// R = |F_A| / |F_AB + F_AC| = |F_total| / |F_pair| on body 0.
pub fn screening_ratio(w: &World) -> f64 {
    norm(&w.force.f_total[0]) / norm(&w.force.f_pair[0])
}
