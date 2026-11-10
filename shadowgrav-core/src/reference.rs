//! Reference mode (Spec 4.4, Traceability D5/E11): a brute-force all-rays evaluation of eq. 3, O(N^3 * samples), for
//! validation at N <= 50. It shares NOTHING with the production quadrature: no HEALPix, no overlap graph, no pair closed
//! form, no cell rule. It integrates the deficit directly,
//!     F_k = Gamma_k * Int n D_k(n) dOmega,   D_k = -expm1(-sum_j beta_j L_j(n)),   Gamma_k = xi Pi A_eff,k / (4 pi),
//! which is the shadow force with mutual shadowing included (V0, alpha_bg = 0). NOTE: this is the same force law as the
//! engine, evaluated by a different method; it is NOT a Newtonian N-body sum (that is a different force: it lacks the
//! saturation S^2 and the overlap correction, so it could validate nothing).
//!
//! Method. Every sky direction with D > 0 lies inside at least one occluder disc. Each disc j is sampled in polar coordinates
//! about its centre direction, with the substitution sin(psi) = sin(theta_j) sin(chi), which removes the chord's square-root
//! edge (the integrand is smooth in chi). A direction covered by several discs is shared out by chord weight
//! beta_j L_j / sum_i beta_i L_i, so each direction is counted exactly once and the discs' mutual edges are the only
//! non-smooth part. The result is returned together with a resolution error estimate (the difference to a grid refined 2x).
//!
//! Scope (anything else returns an error instead of a wrong answer): Mode::V0, alpha_bg = 0, bodies only (no probes),
//! every pair exterior (d_kj > a_j). Interior pairs, velocity modes and Seeliger attenuation are not implemented here.
use crate::geom::*;
use crate::mathx;
use crate::state::World;
use crate::types::*;
use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct ReferenceResult {
    /// Force on every body, same units and sign convention as `ForceOutput::f_total`.
    pub f: Vec<V3>,
    /// max over bodies of |F(2n) - F(n)| / max_k |F_k(2n)|: the resolution error of the reference itself.
    pub est_rel_err: f64,
    /// Number of (sample x occluder) deficit evaluations at the fine resolution (cost indicator).
    pub evaluations: u64,
}

/// Direct evaluation at one resolution (n_chi x n_phi samples per occluder disc, midpoint rule).
fn integrate(
    p: &Particles,
    dv: &Derived,
    g: &Globals,
    n_chi: usize,
    n_phi: usize,
) -> (Vec<V3>, u64) {
    let n = p.x.len();
    let gam_scale = g.xi * g.pi_flux() / (4.0 * PI);
    let mut out = vec![[0.0; 3]; n];
    let mut evals = 0u64;
    for (k, out_k) in out.iter_mut().enumerate() {
        // occluders as seen from k: centre vector, distance, silhouette half-angle, beta
        let occ: Vec<(V3, f64, f64, f64, f64)> = (0..n)
            .filter(|&j| j != k)
            .map(|j| {
                let c = sub(&p.x[j], &p.x[k]);
                let d = norm(&c);
                (c, d, dv.a[j], dv.beta[j], (dv.a[j] / d).asin())
            })
            .collect();
        let mut acc = [0.0; 3];
        for (jj, &(c, d, a, _beta, theta)) in occ.iter().enumerate() {
            let chat = scale(&c, 1.0 / d);
            let (e1, e2) = orthobasis(&chat);
            let (s_th, _) = (theta.sin(), theta.cos());
            for ic in 0..n_chi {
                let chi = 0.5 * PI * (ic as f64 + 0.5) / n_chi as f64;
                let (sx, cx) = (chi.sin(), chi.cos());
                let sin_psi = s_th * sx;
                let cos_psi = (1.0 - sin_psi * sin_psi).sqrt();
                // dOmega = sin(psi) dpsi dphi,  dpsi = s_th cos(chi) dchi / cos(psi)
                let dom_chi = sin_psi * s_th * cx / cos_psi * (0.5 * PI / n_chi as f64);
                for ip in 0..n_phi {
                    let phi = 2.0 * PI * (ip as f64 + 0.5) / n_phi as f64;
                    let (sp, cp) = mathx::sin_cos(phi);
                    let nvec = [
                        cos_psi * chat[0] + sin_psi * (cp * e1[0] + sp * e2[0]),
                        cos_psi * chat[1] + sin_psi * (cp * e1[1] + sp * e2[1]),
                        cos_psi * chat[2] + sin_psi * (cp * e1[2] + sp * e2[2]),
                    ];
                    // total optical depth along nvec and the share belonging to disc jj
                    let (mut tot, mut own) = (0.0, 0.0);
                    for (i2, &(c2, _d2, a2, b2, _t2)) in occ.iter().enumerate() {
                        let t = dot(&nvec, &c2);
                        if t <= 0.0 {
                            continue;
                        }
                        let imp2 = dot(&c2, &c2) - t * t;
                        if imp2 < a2 * a2 {
                            let l = b2 * 2.0 * (a2 * a2 - imp2).sqrt();
                            tot += l;
                            if i2 == jj {
                                own = l;
                            }
                        }
                    }
                    evals += occ.len() as u64;
                    if tot <= 0.0 {
                        continue;
                    }
                    let w = -mathx::expm1(-tot) * (own / tot) * dom_chi * (2.0 * PI / n_phi as f64);
                    acc = add(&acc, &scale(&nvec, w));
                }
            }
            let _ = a;
        }
        *out_k = scale(&acc, gam_scale * dv.a_eff[k]);
    }
    (out, evals)
}

fn check_scope(p: &Particles, dv: &Derived, g: &Globals, mode: Mode) -> Result<(), CoreError> {
    if mode != Mode::V0 {
        return Err(CoreError::Config("reference mode: V0 only"));
    }
    if g.alpha_bg != 0.0 {
        return Err(CoreError::Config("reference mode: alpha_bg = 0 only"));
    }
    if p.kind.iter().any(|k| *k != Kind::Body) {
        return Err(CoreError::Config("reference mode: bodies only (no probes)"));
    }
    let n = p.x.len();
    for k in 0..n {
        for j in 0..n {
            if j != k && norm(&sub(&p.x[j], &p.x[k])) <= dv.a[j] {
                return Err(CoreError::Config(
                    "reference mode: exterior pairs only (interior target)",
                ));
            }
        }
    }
    Ok(())
}

/// All-rays reference forces for every body. `n_chi` x `n_phi` samples per occluder disc; the error estimate uses 2x refinement.
pub fn reference_forces(
    p: &Particles,
    dv: &Derived,
    g: &Globals,
    mode: Mode,
    n_chi: usize,
    n_phi: usize,
) -> Result<ReferenceResult, CoreError> {
    check_scope(p, dv, g, mode)?;
    if p.x.len() > 50 {
        return Err(CoreError::Config("reference mode is specified for N <= 50"));
    }
    let (coarse, _) = integrate(p, dv, g, n_chi, n_phi);
    let (fine, evaluations) = integrate(p, dv, g, 2 * n_chi, 2 * n_phi);
    // normalised by the LARGEST force: a body whose force vanishes by symmetry must not dominate the estimate
    let scale_f = fine.iter().map(norm).fold(0.0f64, f64::max);
    let mut est = 0.0f64;
    if scale_f > 0.0 {
        for k in 0..fine.len() {
            est = est.max(norm(&sub(&fine[k], &coarse[k])) / scale_f);
        }
    }
    Ok(ReferenceResult {
        f: fine,
        est_rel_err: est,
        evaluations,
    })
}

/// Spec 6 item 11: the brute-force comparison error of the live engine forces, for a world in reference scope (N <= 50).
#[derive(Clone, Debug)]
pub struct ReferenceCheck {
    /// max over bodies of |F_engine - F_reference| / max_k |F_reference,k|
    pub max_rel_diff: f64,
    /// the reference's own resolution error estimate; differences below this are not meaningful
    pub reference_est_err: f64,
    pub evaluations: u64,
}
pub fn reference_check(w: &World, n_chi: usize, n_phi: usize) -> Result<ReferenceCheck, CoreError> {
    let r = reference_forces(&w.p, &w.dv, &w.g, w.opt.mode, n_chi, n_phi)?;
    let scale_f = r.f.iter().map(norm).fold(0.0f64, f64::max); // largest force: see `reference_forces`
    let mut worst = 0.0f64;
    if scale_f > 0.0 {
        for k in 0..r.f.len() {
            worst = worst.max(norm(&sub(&w.force.f_total[k], &r.f[k])) / scale_f);
        }
    }
    Ok(ReferenceCheck {
        max_rel_diff: worst,
        reference_est_err: r.est_rel_err,
        evaluations: r.evaluations,
    })
}
