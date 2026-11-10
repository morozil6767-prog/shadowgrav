//! Eq. 8 (ambient drag, unshielded, exact in e).
use crate::geom::*;
pub fn h_fn(e: f64) -> f64 {
    if e <= 1.0 {
        4.0 / 3.0 * e + 4.0 / 15.0 * e * e * e
    } else {
        e * e + 2.0 / 3.0 - 1.0 / (15.0 * e * e)
    }
}
/// F_dr = -xi Pi A_eff H(e) eps_hat
pub fn f_dr(xi: f64, pi_flux: f64, a_eff: f64, eps: &V3) -> V3 {
    let e = norm(eps);
    if e == 0.0 {
        return [0.0; 3];
    }
    scale(eps, -xi * pi_flux * a_eff * h_fn(e) / e)
}
