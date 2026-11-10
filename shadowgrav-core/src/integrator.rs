//! Time integration: Velocity-Verlet (eq. 11), fixed-point solve for V1/V2 (eq. 12),
//! adaptive global step (eq. 13). Calls `evaluate_forces` only; never touches its internals.
use crate::evaluate_forces;
use crate::geom::*;
use crate::state::{accelerations, SimError, StepControl, World};
use crate::sum::Neum3;
use crate::types::*;
use crate::velocity::f_dr;

/// Eq. 12 termination (Contract): tolerance 1e-12, at most 6 iterations.
pub const FP_TOL: f64 = 1e-12;
pub const FP_MAX_ITERS: u32 = 6;
const FP_DENOM_GUARD: f64 = 1e-30;
/// Adaptive-step cap on h*gamma_max (Addendum 2, replaces the 0.1 of Spec eq. 13): keeps eq. 12 convergent.
pub const GAMMA_H_CAP: f64 = 0.01;

#[derive(Clone, Copy, Debug)]
pub struct StepStats {
    pub h: f64,
    /// Number of eq. 12 iterations used (0 for V0).
    pub fp_iters: u32,
    /// Last max_k |v^(i+1) - v^(i)| / (|v^(i+1)| + 1e-30).
    pub fp_residual: f64,
    pub fp_converged: bool,
    /// Calls to evaluate_forces during this step (V1 shortcut: 1; V2: up to 6; V0: 1).
    pub force_evals: u32,
}

fn ensure_finite(v: &[V3]) -> Result<(), SimError> {
    for (i, x) in v.iter().enumerate() {
        if !(x[0].is_finite() && x[1].is_finite() && x[2].is_finite()) {
            return Err(SimError::NonFiniteState { index: i });
        }
    }
    Ok(())
}

fn set_body_velocities(p: &mut Particles, v: &[V3]) {
    for (k, vel) in v.iter().enumerate() {
        if p.kind[k] == Kind::Body {
            p.v[k] = *vel;
        }
    }
}

/// Eq. 13. Global step from the stored state/force at the start of the step.
/// l_k = max(a_k, min_j d_kj); eta_t = 0.005 (Contract). O(N^2) reference neighbour search.
pub fn adaptive_h(w: &World, h_max: f64, eta_t: f64) -> f64 {
    let n = w.p.x.len();
    let vel = w.opt.mode != Mode::V0;
    let mut h = h_max;
    let mut gmax: f64 = 0.0;
    for k in 0..n {
        if w.p.kind[k] != Kind::Body {
            continue;
        }
        let mut dmin = f64::INFINITY;
        for j in 0..n {
            if j == k || w.p.kind[j] != Kind::Body {
                continue;
            }
            let d = norm(&sub(&w.p.x[j], &w.p.x[k]));
            if d < dmin {
                dmin = d;
            }
        }
        let l = if dmin.is_finite() {
            w.dv.a[k].max(dmin)
        } else {
            w.dv.a[k]
        };
        let am = norm(&w.acc[k]);
        if am > 0.0 {
            h = h.min(eta_t * (l / am).sqrt());
        }
        let vm = norm(&w.p.v[k]);
        if vm > 0.0 {
            h = h.min(eta_t * l / vm);
        }
        if vel {
            let u = norm(&sub(&w.p.v[k], &w.g.w));
            if u > 0.0 {
                let gam = norm(&w.force.f_dr[k]) / (w.p.m[k] * u); // gamma_k (Spec 3.3)
                if gam > gmax {
                    gmax = gam;
                }
            }
        }
    }
    // Spec eq. 13 says 0.1/gamma_max. The eq. 12 fixed point contracts by ~h*gamma/2 per iteration, so 1e-12 in
    // <= 6 iterations needs h*gamma <~ 0.02 (0.05^6 ~ 1.6e-8 never reaches it). Addendum 2 tightens the cap.
    if gmax > 0.0 {
        h = h.min(GAMMA_H_CAP / gmax);
    }
    if h.is_finite() && h > 0.0 {
        h
    } else {
        h_max
    }
}

pub fn choose_h(w: &World) -> f64 {
    match w.cfg.step {
        StepControl::Fixed { h } => h,
        StepControl::Adaptive { h_max, eta_t } => adaptive_h(w, h_max, eta_t),
    }
}

/// One step with the configured step control.
pub fn advance(w: &mut World) -> Result<StepStats, SimError> {
    w.validate()?; // validate before adaptive_h indexes public world arrays
    let h = choose_h(w);
    advance_by(w, h)
}

/// One step of size h. Atomic: on any error x and v are restored and force/acc/time are untouched.
pub fn advance_by(w: &mut World, h: f64) -> Result<StepStats, SimError> {
    w.validate()?;
    if crate::mathx::not_gt(h, 0.0) || !h.is_finite() {
        return Err(SimError::BadConfig("h must be positive and finite"));
    }
    let x_old = w.p.x.clone();
    let v_old = w.p.v.clone();
    match advance_inner(w, h) {
        Ok((stats, force, acc)) => {
            commit(w, stats, force, acc);
            Ok(stats)
        }
        Err(e) => {
            w.p.x = x_old;
            w.p.v = v_old;
            Err(e)
        }
    }
}

fn commit(w: &mut World, stats: StepStats, force: ForceOutput, acc: Vec<V3>) {
    w.time.add(stats.h);
    w.step_count += 1;
    w.flags.cap_hit_total += force.status.cap_hit_count;
    if !stats.fp_converged {
        w.flags.fp_nonconverged_steps += 1;
    }
    if stats.fp_residual.is_finite() && stats.fp_residual > w.flags.fp_max_residual {
        w.flags.fp_max_residual = stats.fp_residual;
    }
    w.force = force;
    w.acc = acc;
    w.last_step = Some(stats);
    if w.step_count % w.cfg.diag_every == 0 {
        w.record_conservation();
    }
}

/// V1 only. F_sh is position-only, so after the first full evaluation at x_{n+1} only the
/// closed-form eq. 8 drag changes with v. Mirrors force.rs term-for-term (same expressions,
/// same summation order), so the result is bit-identical to a full evaluation.
fn refresh_drag(
    o: &mut ForceOutput,
    p: &Particles,
    dv: &Derived,
    g: &Globals,
) -> Result<(), SimError> {
    let pi_flux = g.pi_flux();
    for k in 0..p.x.len() {
        let eps = if p.kind[k] == Kind::Body {
            scale(&sub(&p.v[k], &g.w), 1.0 / g.v_g)
        } else {
            [0.0; 3]
        };
        let fdr = f_dr(g.xi, pi_flux, dv.a_eff[k], &eps);
        let mut tot = Neum3::default();
        tot.add(&o.f_pair[k]);
        tot.add(&o.f_corr[k]);
        tot.add(&fdr);
        tot.add(&o.f_ab[k]);
        let ft = tot.value();
        if !(fdr.iter().all(|x| x.is_finite()) && ft.iter().all(|x| x.is_finite())) {
            return Err(SimError::Core(CoreError::NonFinite { target: k }));
        }
        o.f_dr[k] = fdr;
        o.f_total[k] = ft;
    }
    Ok(())
}

fn advance_inner(w: &mut World, h: f64) -> Result<(StepStats, ForceOutput, Vec<V3>), SimError> {
    let n = w.p.x.len();
    let mode = w.opt.mode;
    let hh = 0.5 * h;

    // Eq. 11: half kick with a_n, then drift with the half-step velocity.
    let mut v_half = vec![[0.0; 3]; n];
    for (k, v_row) in v_half.iter_mut().enumerate() {
        if w.p.kind[k] != Kind::Body {
            continue;
        }
        for (c, v_comp) in v_row.iter_mut().enumerate() {
            *v_comp = w.p.v[k][c] + hh * w.acc[k][c];
            w.p.x[k][c] += h * *v_comp;
        }
    }
    ensure_finite(&w.p.x)?;
    let mut stats = StepStats {
        h,
        fp_iters: 0,
        fp_residual: 0.0,
        fp_converged: true,
        force_evals: 0,
    };

    // ---- V0: eq. 11, one force evaluation ----
    if mode == Mode::V0 {
        set_body_velocities(&mut w.p, &v_half); // irrelevant to V0 forces
        let out = evaluate_forces(&w.p, &w.dv, &w.g, &w.opt)?;
        stats.force_evals = 1;
        let a_new = accelerations(&w.p, &out.f_total);
        let mut v_new = v_half;
        for k in 0..n {
            if w.p.kind[k] != Kind::Body {
                continue;
            }
            for c in 0..3 {
                v_new[k][c] += hh * a_new[k][c];
            }
        }
        set_body_velocities(&mut w.p, &v_new);
        ensure_finite(&w.p.v)?;
        return Ok((stats, out, a_new));
    }

    // ---- V1/V2: eq. 12 fixed point ----
    // v^(0) = v_{n+1/2} + (h/2) a_n ;  v^(i+1) = v_{n+1/2} + (h/2) a(x_{n+1}, v^(i))
    let mut v_iter = vec![[0.0; 3]; n];
    for k in 0..n {
        if w.p.kind[k] != Kind::Body {
            continue;
        }
        for c in 0..3 {
            v_iter[k][c] = v_half[k][c] + hh * w.acc[k][c];
        }
    }
    let mut out: Option<ForceOutput> = None;
    let mut a_last = vec![[0.0; 3]; n];
    let mut converged = false;
    let mut resid = f64::INFINITY;
    for it in 0..FP_MAX_ITERS {
        set_body_velocities(&mut w.p, &v_iter);
        let full = out.is_none() || mode == Mode::V2 || !w.cfg.v1_shortcut;
        if full {
            out = Some(evaluate_forces(&w.p, &w.dv, &w.g, &w.opt)?);
            stats.force_evals += 1;
        } else if let Some(o) = out.as_mut() {
            refresh_drag(o, &w.p, &w.dv, &w.g)?;
        }
        let o = out.as_ref().expect("force output present");
        a_last = accelerations(&w.p, &o.f_total);
        resid = 0.0;
        for k in 0..n {
            if w.p.kind[k] != Kind::Body {
                continue;
            }
            let mut vn = [0.0; 3];
            for (c, slot) in vn.iter_mut().enumerate() {
                *slot = v_half[k][c] + hh * a_last[k][c];
            }
            // Spec 3.2 denominator |v| + 1e-30, made robust for near-stationary bodies by adding the kick
            // scale (h/2)|a| (changes the criterion by O(h) relative for moving bodies; avoids 0/0 noise
            // for a body that sits at v ~ 1e-17 with zero net force).
            let r =
                norm(&sub(&vn, &v_iter[k])) / (norm(&vn) + hh * norm(&a_last[k]) + FP_DENOM_GUARD);
            if !r.is_finite() {
                return Err(SimError::NonFiniteState { index: k });
            }
            if r > resid {
                resid = r;
            }
            v_iter[k] = vn;
        }
        stats.fp_iters = it + 1;
        if resid < FP_TOL {
            converged = true;
            break;
        }
    }
    stats.fp_residual = resid;
    stats.fp_converged = converged;
    // v_{n+1} = v^(i+1); stored force/acceleration are the last *evaluated* ones (Spec 3.2),
    // i.e. at v^(i); they differ from the converged velocity by < 1e-12 relative.
    set_body_velocities(&mut w.p, &v_iter);
    ensure_finite(&w.p.v)?;
    let out = out.expect("at least one iteration ran");
    Ok((stats, out, a_last))
}
