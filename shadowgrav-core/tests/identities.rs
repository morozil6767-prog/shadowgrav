//! Stage 2/3. Pure identities and single-step force checks (no time integration).
use shadowgrav_core::derive::a_fun;
use shadowgrav_core::geom::*;
use shadowgrav_core::pair::*;
use shadowgrav_core::*;
use std::f64::consts::PI;

const RHO: f64 = 2.39e5;
fn rad() -> f64 { (3.0 / (4.0 * PI * RHO)).cbrt() }
/// m = 1  =>  tau_c = 3 kappa0 / (2 pi a^2)  =>  kappa0 = 2 pi a^2 tau / 3
fn glob(tau: f64) -> Globals { let a = rad(); let mut g = Globals::defaults(); g.kappa0 = 2.0 * PI * a * a * tau / 3.0; g }
fn bodies(xs: &[V3], vs: &[V3]) -> Particles {
    let n = xs.len();
    Particles { x: xs.to_vec(), v: vs.to_vec(), m: vec![1.0; n], rho: vec![RHO; n], delta: vec![0.0; n], kind: vec![Kind::Body; n] }
}
fn opt(mode: Mode) -> Options { Options { mode, instrumented: false, force_gl: false, leaf_budget: 0, validation: None,
                                          parallel: true, brute_candidates: false, thin_fast: false, thin_theta: 0.5 } }
fn rel(x: f64, y: f64) -> f64 { (x - y).abs() / y.abs().max(1e-300) }
fn run(p: &Particles, g: &Globals, o: &Options) -> ForceOutput { let d = derive(p, g).unwrap(); evaluate_forces(p, &d, g, o).unwrap() }
fn pair_at(dr: f64) -> Particles { bodies(&[[0.0; 3], [dr * rad(), 0.0, 0.0]], &[[0.0; 3]; 2]) }

#[test] fn saturation_function() {
    for &t in &[0.3, 0.7, 0.999999] {
        let (s, ser) = a_fun(t); assert!(ser);
        assert!(rel(s, 1.0 - 2.0 / (t * t) * (1.0 - (1.0 + t) * (-t).exp())) < 1e-12);
    }
    for &(t, s_ref) in &[(0.1, 0.96348), (1.0, 0.70728), (10.0, 0.14700)] {
        assert!((1.5 * a_fun(t).0 / t - s_ref).abs() < 6e-6);
    }
    let (t, (a, _)) = (1e-6, a_fun(1e-6));
    assert!(rel(1.5 * a / t, 1.0 - 3.0 * t / 8.0) < 1e-12);            // S = 1 - 3 tau/8 + tau^2/10
}
#[test] fn aeff_identity_and_thin_force() {                            // A_eff = kappa0 m^g exactly
    let g = glob(1e-6); let p = pair_at(100.0);
    let d = derive(&p, &g).unwrap();
    assert!(rel(d.a_eff[0], g.kappa0 * d.m_g[0]) < 1e-12);
    let f = evaluate_forces(&p, &d, &g, &opt(Mode::V0)).unwrap();
    let dd = 100.0 * rad();
    assert!(rel(f.f_total[0][0], g.g_n * d.m_g[0] * d.m_g[1] / (dd * dd)) < 1e-12);
}
#[test] fn saturation_ratio_is_s_squared_and_d_independent() {
    for &tau in &[0.1, 1.0, 10.0] {
        let g = glob(tau);
        for &dr in &[1.5, 5.0, 10.0, 50.0] {
            let p = pair_at(dr); let d = derive(&p, &g).unwrap();
            let f = evaluate_forces(&p, &d, &g, &opt(Mode::V0)).unwrap();
            let dd = dr * rad();
            assert!(rel(f.f_total[0][0] * dd * dd, d.s[0] * d.s[1]) < 1e-12);
        }
    }
}
#[test] fn large_tau_limit() {
    let g = glob(1e7); let p = pair_at(10.0);
    let f = run(&p, &g, &opt(Mode::V0)); let a = rad(); let d = 10.0 * a;
    let want = g.xi * g.pi_flux() * PI * a.powi(4) / (4.0 * d * d);
    assert!(rel(f.f_total[0][0], want) < 1e-9);
}
#[test] fn closed_form_vs_forced_gl() {
    for &tau in &[1e-6, 0.1, 1.0, 10.0, 50.0] { for &dr in &[1.5, 5.0, 50.0] {
        let (g, p) = (glob(tau), pair_at(dr));
        let (mut o, f0) = (opt(Mode::V0), run(&p, &g, &opt(Mode::V0)));
        o.force_gl = true; let f1 = run(&p, &g, &o);
        assert!(rel(f1.f_total[0][0], f0.f_total[0][0]) < 1e-10, "tau {tau} dr {dr}");
    } }
}
#[test] fn reciprocity_and_n1() {
    let g = glob(1.0);
    for &dr in &[0.5, 1.0, 1.5, 7.0] {
        let f = run(&pair_at(dr), &g, &opt(Mode::V0));
        assert!((f.f_total[0][0] + f.f_total[1][0]).abs() <= 1e-12 * f.f_total[0][0].abs().max(1e-300));
    }
    let f = run(&bodies(&[[1.0, 2.0, 3.0]], &[[0.0; 3]]), &g, &opt(Mode::V0));
    assert_eq!(f.f_pair[0], [0.0; 3]); assert_eq!(f.f_corr[0], [0.0; 3]); assert_eq!(f.f_total[0], [0.0; 3]);
}
/// Interior thin field F = G m_j m_k d / a^3. The deviation from the thin limit is O(tau_c), so the
/// 1e-10 Spec tolerance needs tau_c <= ~1e-11 (Python check: 5e-13 at tau = 1e-12; ~1e-8 at tau = 1e-8).
#[test] fn interior_thin_uniform_sphere_field() {
    let g = glob(1e-12); let a = rad();
    let p = bodies(&[[0.0; 3], [0.3 * a, 0.0, 0.0]], &[[0.0; 3]; 2]);
    let d = derive(&p, &g).unwrap();
    let f = evaluate_forces(&p, &d, &g, &opt(Mode::V0)).unwrap();
    assert!(rel(f.f_total[0][0], g.g_n * d.m_g[0] * d.m_g[1] * 0.3 * a / a.powi(3)) < 1e-10);
}
#[test] fn continuity_at_contact() {
    for &tau in &[0.1, 10.0, 50.0] {
        let g = glob(tau); let a = rad();
        let fm = run(&bodies(&[[0.0; 3], [a * (1.0 - 1e-9), 0.0, 0.0]], &[[0.0; 3]; 2]), &g, &opt(Mode::V0)).f_total[0][0];
        let fp = run(&bodies(&[[0.0; 3], [a * (1.0 + 1e-9), 0.0, 0.0]], &[[0.0; 3]; 2]), &g, &opt(Mode::V0)).f_total[0][0];
        assert!(rel(fm, fp) < 1e-6, "tau {tau}: {}", rel(fm, fp));       // Python ref: 5e-8 at tau = 50
    }
}
#[test] fn e_at_zero_eps_equals_p_chat() {
    let q = Quad::new(32); let mut nodes = vec![]; let mut n2 = vec![];
    let a = 0.01; let chat = [0.6, 0.0, 0.8];
    for &tau in &[1e-3, 1.0, 10.0, 50.0] {
        for &dr in &[1.0 + 1e-9, 1.5, 5.0, 50.0, 1.0, 0.5, 0.0999] {
            let (d, beta) = (dr * a, tau / (2.0 * a));
            make_m_nodes(d, a, beta, &q, &mut nodes);
            let (e, p0) = pair_e(&nodes, beta, &chat, &[0.0; 3], &[0.0; 3], 32);
            // Interior reference: cancellation-free paired pair_p (N12). The signed-node P0 and E carry
            // ~1e-15 ABSOLUTE error (unit-sphere integral = 2*pi), which is not 1e-13 relative when P is
            // exponentially small (large tau, d << a); so interior tolerance is absolute on the 2*pi scale.
            let pref = if d > a { PI * a * a * a_fun(tau).0 / (d * d) }
                       else { pair_p(d, a, beta, 0.0, 0.0, true, &q, &mut n2) };
            let scale = if d > a { pref } else { 2.0 * PI };
            assert!((p0 - pref).abs() < 1e-13 * scale, "P0 tau {tau} dr {dr}: {:e}", (p0 - pref).abs() / scale);
            for i in 0..3 { assert!((e[i] - pref * chat[i]).abs() < 1e-13 * scale, "E tau {tau} dr {dr}"); }
        }
    }
}
#[test] fn seeliger_single_pair() {
    let q = Quad::new(32); let mut s = vec![];
    let (a, tau, alpha, d) = (0.01, 1e-6, 0.05, 1.0);
    let beta = tau / (2.0 * a);
    let p0 = pair_p(d, a, beta, 0.0, PI * a * a * a_fun(tau).0, false, &q, &mut s);
    let pa = pair_p(d, a, beta, alpha, 0.0, false, &q, &mut s);
    assert!((pa / p0 / (-alpha * d).exp() - 1.0).abs() < 2e-3);
}
#[test] fn sweep_matches_alpha_zero_limit() {
    let iv = [(0.5, 1.5, 3.0), (1.0, 2.0, 2.0)];
    let tau_tot: f64 = 3.0 * 1.0 + 2.0 * 1.0;                // beta_i * L_i  (L = 1.0 each)
    assert!((deficit::sweep(&iv, 1e-14) - (1.0 - (-tau_tot).exp())).abs() < 1e-10);
}
#[test] fn v2_zero_velocity_reduces_to_v0() {
    let g = glob(1.0);
    let p = bodies(&[[0.0; 3], [10.0 * rad(), 0.0, 0.0], [20.0 * rad(), 0.0, 0.0]], &[[0.0; 3]; 3]);
    let (f0, f2) = (run(&p, &g, &opt(Mode::V0)), run(&p, &g, &opt(Mode::V2)));
    // Global scale: the middle body B has F_B = 0 by symmetry, so a per-body scale would be ~1e-30.
    let scale = (0..3).map(|k| norm(&f0.f_total[k])).fold(0.0, f64::max);
    for k in 0..3 { for i in 0..3 {
        assert!((f2.f_total[k][i] - f0.f_total[k][i]).abs() < 1e-13 * scale, "k {k} i {i}");
        assert!(f2.f_ab[k][i].abs() < 1e-13 * scale, "f_ab k {k} i {i}");
    } }
}
#[test] fn v2_matches_eq10_thin_pair() {
    let mut g = glob(1e-6); g.v_g = 1e3;
    let (ek, ej) = ([0.8e-3, 0.4e-3, -0.6e-3], [-0.5e-3, 0.7e-3, 0.3e-3]);
    let chat = [0.6, 0.0, 0.8];
    let p = bodies(&[[0.0; 3], chat], &[scale(&ek, g.v_g), scale(&ej, g.v_g)]);
    let d = derive(&p, &g).unwrap();
    let f = evaluate_forces(&p, &d, &g, &opt(Mode::V2)).unwrap();
    let mut got = [0.0; 3]; for i in 0..3 { got[i] = f.f_pair[0][i] + f.f_corr[0][i] + f.f_ab[0][i]; }
    let pref = g.g_n * d.m_g[0] * d.m_g[1];
    let c = 1.0 + dot(&chat, &add(&scale(&ej, 3.0), &ek));
    let mut want = [0.0; 3]; for i in 0..3 { want[i] = pref * (chat[i] * c + ek[i] - ej[i]); }
    let emax = norm(&ej).max(norm(&ek));
    assert!(norm(&sub(&got, &want)) / norm(&want) < 10.0 * emax * emax);    // Spec (d3)
}
#[test] fn isolated_sphere_is_inert() {                                // pins hardening Q1
    let (g, a) = (glob(20.0), rad());
    let base = [[0.0; 3], [10.0 * a, 0.0, 0.0], [20.0 * a, 0.0, 0.0]];
    let f3 = run(&bodies(&base, &[[0.0; 3]; 3]), &g, &opt(Mode::V0));
    let mut four = base.to_vec(); four.push([0.0, 10.0 * a, 0.0]);
    let f4 = run(&bodies(&four, &[[0.0; 3]; 4]), &g, &opt(Mode::V0));
    assert_eq!(f4.f_corr[0], f3.f_corr[0]);                            // bitwise
}
#[test] fn disjoint_has_empty_correction() {
    let (g, a) = (glob(20.0), rad());
    let f = run(&bodies(&[[0.0; 3], [10.0 * a, 0.0, 0.0], [20.0 * a, 10.0 * a, 0.0]], &[[0.0; 3]; 3]), &g, &opt(Mode::V0));
    assert_eq!(f.f_corr[0], [0.0; 3]);
}
#[test] fn screening_standard_mode_provisional() {                     // test (c), 5e-3 clause only
    let (g, a) = (glob(20.0), rad());
    let f = run(&bodies(&[[0.0; 3], [10.0 * a, 0.0, 0.0], [20.0 * a, 0.0, 0.0]], &[[0.0; 3]; 3]), &g, &opt(Mode::V0));
    let r = norm(&f.f_total[0]) / norm(&f.f_pair[0]);
    assert!((r - 0.8).abs() < 5e-3, "R = {r}");
    assert!(norm(&f.f_total[1]) < 1e-9 * norm(&f.f_total[0]));
    assert!(norm(&add(&f.f_total[0], &f.f_total[2])) < 1e-9 * norm(&f.f_total[0]));
}

/// N12: interior pair at large tau / small d, vs 40-digit mpmath references (a = 0.01, alpha = 0).
/// Signed-sum evaluation misses these by 9e-11 and 1.2e-6; the paired form is at ~4e-16.
#[test] fn interior_pair_cancellation_free() {
    let q = Quad::new(32); let mut s = vec![]; let a = 0.01;
    for &(dr, tau, want) in &[(0.5, 50.0, 2.932191776417631e-6), (0.0999, 50.0, 2.7008924919953155e-10),
                              (0.5, 10.0, 0.15926139060497088)] {
        let p = pair_p(dr * a, a, tau / (2.0 * a), 0.0, 0.0, true, &q, &mut s);
        assert!(rel(p, want) < 1e-13, "dr {dr} tau {tau}: {:e}", rel(p, want));
    }
}
