//! Pair terms: P_kj (eqs. 5, 6a, 6b), eq.-20 moments, E_kj (Addendum I.3).
//!
//! Node families (same integral, different variable; all D evaluated by eq. 5):
//!  * exterior_nodes_phi : eq. 6a variable phi (reparametrised u = pi/2 - phi). Smooth weight for P.
//!    Graded toward the limb (u -> 0) when tau_node > 4 (boundary layer cos(phi) ~ 1/tau).
//!  * exterior_nodes_sinh: for Omega, tau_bar and E, whose natural weight is singular near contact.
//!    With r = a/d, kappa = sqrt(1-r^2)/r and x = cos(phi) = sin(psi), substitute x = kappa sinh s:
//!        cos(theta') = sqrt(1-r^2) cosh(s),  dOmega/(2 pi) = r x ds.
//!    Check: for D = 1 the integral is sqrt(1-r^2)(cosh s_max - 1) = 1 - sqrt(1-r^2).  Smooth for all r<1.
//!  * interior_nodes     : eq. 6b panels (verbatim) + extra breakpoints (kink at |mu| = eps, grading).
use crate::deficit::d_single;
use crate::geom::*;
use crate::gl::Gl;
use crate::mathx;
use crate::sum::{Neum, Neum3};
use std::f64::consts::{FRAC_PI_2, PI};

pub struct Quad { pub gl: Gl, pub gl16: Gl }
impl Quad { pub fn new(n_gl: usize) -> Quad { Quad { gl: Gl::new(n_gl), gl16: Gl::new(16) } } }

/// Polar node of a sphere's sky disc.  P = 2pi sum w_p D ; Omega = 2pi sum w_om D ; w_p = w_om cos_th.
pub struct Node { pub l: f64, pub s1: f64, pub w_p: f64, pub w_om: f64, pub cos_th: f64, pub sin_th: f64 }

const PANELS: [f64; 9] = [-1.0, -0.5, -0.1, -0.02, 0.0, 0.02, 0.1, 0.5, 1.0];

/// Number of dyadic grading levels for a transition layer of width ~1/tau. 0 when tau <= 4.
fn grade_levels(tau: f64) -> u32 {
    let (mut k, mut t) = (0u32, tau / 4.0);
    while t > 1.0 && k < 40 { t *= 0.5; k += 1; }
    k
}

/// Eq. 6a, exterior. u = pi/2 - phi, so sin(phi) = cos(u), cos(phi) = sin(u) (no loss at the limb).
pub fn exterior_nodes_phi(d: f64, a: f64, tau_node: f64, gl: &Gl, out: &mut Vec<Node>) {
    out.clear();
    let r = a / d;
    let kk = grade_levels(tau_node);
    let mut br = vec![0.0];
    for k in (0..=kk).rev() { br.push(FRAC_PI_2 * 0.5f64.powi(k as i32)); }
    for p in 0..br.len() - 1 {
        let (hw, mid) = (0.5 * (br[p + 1] - br[p]), 0.5 * (br[p + 1] + br[p]));
        for i in 0..gl.x.len() {
            let (u, w) = (mid + hw * gl.x[i], hw * gl.w[i]);
            let (su, cu) = mathx::sin_cos(u);
            let sp = cu;                                          // sin(phi) = b/a
            let cos_th = ((1.0 - r * sp) * (1.0 + r * sp)).sqrt();
            let sq = ((d - a * sp) * (d + a * sp)).sqrt();
            let s1 = (d - a) * (d + a) / (sq + a * su);           // eq. 6a s1, cancellation-free
            let wp = w * r * r * sp * su;
            out.push(Node { l: 2.0 * a * su, s1, w_p: wp, w_om: wp / cos_th, cos_th, sin_th: r * sp });
        }
    }
}

/// Exterior nodes for Omega / tau_bar / E (see module header). Sub-panels at most 2 wide in s.
pub fn exterior_nodes_sinh(d: f64, a: f64, tau_node: f64, gl16: &Gl, out: &mut Vec<Node>) {
    out.clear();
    let r = a / d;
    let om = (1.0 - r) * (1.0 + r);                               // 1 - r^2
    let kappa = om.sqrt().max(1e-100) / r;
    let kk = grade_levels(tau_node);
    let mut xs = vec![0.0];                                       // breakpoints in x = sin(psi) = cos(phi)
    for k in (0..=kk).rev() { xs.push(0.5f64.powi(k as i32)); }
    for p in 0..xs.len() - 1 {
        let (s_lo, s_hi) = (mathx::asinh(xs[p] / kappa), mathx::asinh(xs[p + 1] / kappa));
        let np = (((s_hi - s_lo) / 2.0).ceil() as usize).max(1);
        for q in 0..np {
            let lo = s_lo + (s_hi - s_lo) * q as f64 / np as f64;
            let hi = s_lo + (s_hi - s_lo) * (q + 1) as f64 / np as f64;
            let (hw, mid) = (0.5 * (hi - lo), 0.5 * (hi + lo));
            for i in 0..gl16.x.len() {
                let (s, ws) = (mid + hw * gl16.x[i], hw * gl16.w[i]);
                let x = (kappa * mathx::sinh(s)).min(1.0);        // sin(psi) = cos(phi) = L/(2a)
                let cpsi = ((1.0 - x) * (1.0 + x)).sqrt();        // sin(phi) = b/a
                let cos_th = (om + (r * x) * (r * x)).sqrt();     // = sqrt(1-r^2) cosh(s)
                let w_om = ws * r * x;
                let s1 = (d - a) * (d + a) / (d * cos_th + a * x);
                out.push(Node { l: 2.0 * a * x, s1, w_p: w_om * cos_th, w_om, cos_th, sin_th: r * cpsi });
            }
        }
    }
}

/// Eq. 6b, interior/contact. mu = cos(theta'); L = s2(mu) in the cancellation-free form.
/// Extra breakpoints (Architecture R3/R4): kink scale eps = sqrt(a^2-d^2)/d, and 2^-k grading.
pub fn interior_nodes(d: f64, a: f64, tau_node: f64, gl16: &Gl, out: &mut Vec<Node>) {
    out.clear();
    let mut br: Vec<f64> = PANELS.to_vec();
    if d > 0.0 {
        let eps = ((a - d) * (a + d)).sqrt() / d;
        if eps > 0.0 { let mut e = eps; while e < 0.5 { br.push(e); br.push(-e); e *= 4.0; } }
    }
    for k in 1..=grade_levels(tau_node) { let e = 0.5f64.powi(k as i32); br.push(e); br.push(-e); }
    br.sort_by(|x, y| x.total_cmp(y));
    br.dedup_by(|later, earlier| (*later - *earlier).abs() < 1e-15);
    for p in 0..br.len() - 1 {
        let (hw, mid) = (0.5 * (br[p + 1] - br[p]), 0.5 * (br[p + 1] + br[p]));
        for i in 0..gl16.x.len() {
            let (mu, w) = (mid + hw * gl16.x[i], hw * gl16.w[i]);
            let sq = ((a - d) * (a + d) + d * d * mu * mu).sqrt();
            let s2 = if mu >= 0.0 { d * mu + sq } else {
                let den = sq - d * mu;
                if den > 0.0 { (a - d) * (a + d) / den } else { 0.0 }
            };
            out.push(Node { l: s2.max(0.0), s1: 0.0, w_p: w * mu, w_om: w, cos_th: mu,
                            sin_th: ((1.0 - mu) * (1.0 + mu)).sqrt() });
        }
    }
}

/// Nodes for P (GL path).  tau_node = 2 a (alpha + beta) = width control for the saturation layer.
pub fn make_p_nodes(d: f64, a: f64, lam: f64, q: &Quad, out: &mut Vec<Node>) {
    if d > a { exterior_nodes_phi(d, a, 2.0 * a * lam, &q.gl, out) } else { interior_nodes(d, a, 2.0 * a * lam, &q.gl16, out) }
}
/// Nodes for Omega / tau_bar / E.
pub fn make_m_nodes(d: f64, a: f64, beta: f64, q: &Quad, out: &mut Vec<Node>) {
    if d > a { exterior_nodes_sinh(d, a, 2.0 * a * beta, &q.gl16, out) } else { interior_nodes(d, a, 2.0 * a * beta, &q.gl16, out) }
}

/// P_kj. Closed form A_eff/d^2 ONLY for exterior pairs with alpha_bg = 0 (Spec 2.3); else GL.
/// d < 1e-12 a => 0 (Spec 1.6).
pub fn pair_p(d: f64, a: f64, beta: f64, alpha: f64, a_eff: f64, force_gl: bool,
              q: &Quad, scratch: &mut Vec<Node>) -> f64 {
    if d < 1e-12 * a { return 0.0; }
    if d > a && alpha == 0.0 && !force_gl { return a_eff / (d * d); }
    let lam = alpha + beta;
    if !(lam > 0.0) { return 0.0; }                                  // eq. 5: D = 0 for lambda = 0
    make_p_nodes(d, a, lam, q, scratch);
    let mut s = Neum::default();
    if d > a {
        for nd in scratch.iter() { s.add(nd.w_p * d_single(alpha, beta, nd.s1, nd.l)); }
    } else {
        // Hardening N12 (eq. 6b, same panels/nodes). Interior P = 2pi int_{-1}^{1} D(mu) mu dmu is a
        // difference of two nearly equal hemispheres when tau is large or d << a (P ~ e^{-tau/2}), so a
        // signed sum loses ~1e-16/P. Exact rewrite: P = 2pi int_0^1 mu [D(mu) - D(-mu)] dmu with
        //   L(mu) - L(-mu) = 2 d mu (exact)   and   s1 < 0 (so eq. 5 has no e^{-alpha s1} factor):
        //   D(mu) - D(-mu) = (beta/lam) e^{-lam L(-mu)} (-expm1(-2 lam d mu)),
        //   L(-mu) = (a^2-d^2) / (sqrt(a^2-d^2+d^2 mu^2) + d mu)   (cancellation-free; 0 at d = a).
        // Only the mu > 0 nodes are used; both factors are small-argument accurate.
        let k2 = (a - d) * (a + d);
        for nd in scratch.iter() {
            let mu = nd.cos_th;
            if mu <= 0.0 { continue; }
            let sq = (k2 + d * d * mu * mu).sqrt();
            let l_minus = k2 / (sq + d * mu);
            let dd = (beta / lam) * mathx::exp(-lam * l_minus) * (-mathx::expm1(-2.0 * lam * d * mu));
            s.add(nd.w_p * dd);                                      // w_p = w*mu > 0 here
        }
    }
    2.0 * PI * s.value()
}

pub struct Moments { pub p: f64, pub omega: f64, pub tau_int: f64 }
/// Omega_kj = int D dOmega ; tau_int = int tau dOmega (tau_bar = tau_int/4pi).
pub fn pair_moments(d: f64, a: f64, beta: f64, alpha: f64, q: &Quad, scratch: &mut Vec<Node>) -> Moments {
    if d < 1e-12 * a {                                            // Spec 1.6: D evaluated with L = a
        return Moments { p: 0.0, omega: 4.0 * PI * d_single(alpha, beta, 0.0, a), tau_int: 4.0 * PI * beta * a };
    }
    make_m_nodes(d, a, beta, q, scratch);
    let (mut sp, mut so, mut st) = (Neum::default(), Neum::default(), Neum::default());
    for nd in scratch.iter() {
        let dd = d_single(alpha, beta, nd.s1, nd.l);
        sp.add(nd.w_p * dd); so.add(nd.w_om * dd); st.add(nd.w_om * beta * nd.l);
    }
    Moments { p: 2.0 * PI * sp.value(), omega: 2.0 * PI * so.value(), tau_int: 2.0 * PI * st.value() }
}

/// Addendum I.3.  E_kj = int g(n;eps_k) D_kj(q_j(n)) dOmega_n
///   = int (lam^2/w) |lam q + Delta|(lam q + Delta) D_kj(q) dOmega_q,
///   Delta = eps_k - eps_j, w = sqrt((q.eps_j)^2 + 1 - e_j^2), lam = q.eps_j + w   (alpha_bg = 0 only).
/// Returns (E_kj, P0) with P0 = int cos(theta') D dOmega on the SAME nodes, so that
/// B_kj = E_kj - P0 chat vanishes identically at eps = 0 (hardening N1).
/// `nodes` must come from make_m_nodes(d, a, beta, ..) for this pair.
pub fn pair_e(nodes: &[Node], beta: f64, chat: &V3, eps_k: &V3, eps_j: &V3, n_az: usize) -> (V3, f64) {
    let e2 = dot(eps_j, eps_j);
    let delta = sub(eps_k, eps_j);
    let (b1, b2) = orthobasis(chat);
    let az: Vec<(f64, f64)> = (0..n_az).map(|m| mathx::sin_cos(2.0 * PI * m as f64 / n_az as f64)).collect();
    let (mut tot, mut p0) = (Neum3::default(), Neum::default());
    for nd in nodes {
        let dd = d_single(0.0, beta, nd.s1, nd.l);
        if dd == 0.0 { continue; }
        p0.add(nd.w_p * dd);
        let mut inner = Neum3::default();
        for &(sn, cs) in &az {
            let qv = [
                nd.cos_th * chat[0] + nd.sin_th * (cs * b1[0] + sn * b2[0]),
                nd.cos_th * chat[1] + nd.sin_th * (cs * b1[1] + sn * b2[1]),
                nd.cos_th * chat[2] + nd.sin_th * (cs * b1[2] + sn * b2[2]),
            ];
            let qe = dot(&qv, eps_j);
            let w = (qe * qe + 1.0 - e2).sqrt();
            let lam = qe + w;
            let v = add(&scale(&qv, lam), &delta);
            inner.add_scaled(lam * lam / w, &scale(&v, norm(&v)));
        }
        tot.add_scaled(2.0 * PI * nd.w_om * dd / n_az as f64, &inner.value());
    }
    (tot.value(), 2.0 * PI * p0.value())
}
