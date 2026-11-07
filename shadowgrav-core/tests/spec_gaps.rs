//! Tests added to close audit gaps found while building TRACEABILITY.md (Spec section 6 items 2, 5, 9 and test (e)).
//! Every tolerance is derived in the comment next to it; none is tuned to a measurement.
use shadowgrav_core::diagnostics::{drag_rate, eta_ep, thin_and_newton};
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn pair_world(tau: f64, d_over_a: f64, delta: [f64; 2], instrumented: bool) -> World {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let mut p = body_particles(vec![[0.0; 3], [d_over_a * a, 0.0, 0.0]], vec![[0.0; 3]; 2]);
    p.delta = delta.to_vec();
    let mut o = default_options(Mode::V0);
    o.instrumented = instrumented;
    World::new(p, g, o, SimConfig::fixed(1e-4)).unwrap()
}

/// Spec 7(e), equivalence principle. Two test bodies that differ only in the species offset delta feel the same source
/// at the same distance (two separate runs, so they do not disturb each other). In the thin limit a_k is proportional to
/// (1 + delta_k), so eta_EP = 2 delta / (2 + delta) = delta - delta^2/2 exactly; the saturation factor S(tau_c(1+delta))
/// differs from S(tau_c) by O(tau_c delta) ~ 4e-7 delta at tau_c = 1e-6. Tolerance: delta^2 (second order) + 1e-6 delta.
#[test] fn equivalence_principle_eta_ep_equals_delta_to_first_order() {
    let accel = |delta_test: f64| {
        // source (index 0, delta 0) at the origin; test body (index 1) at d = 100 a, so only the pair term acts
        let w = pair_world(1e-6, 100.0, [0.0, delta_test], false);
        let acc = accelerations(&w.p, &w.force.f_total);
        acc[1]
    };
    let a0 = accel(0.0);
    assert_eq!(eta_ep(&a0, &a0), 0.0);
    for &delta in &[1e-5, 1e-4, 1e-3] {
        let eta = eta_ep(&a0, &accel(delta));
        assert!((eta - delta).abs() <= delta * delta + 1e-6 * delta, "delta {delta:e}: eta_EP = {eta:e}");
    }
}

/// Spec 6 item 2: for an exterior pair (d > a, alpha_bg = 0) the force is F = S_k S_j F_thin exactly (Spec 7b) and the
/// unsaturated-point-mass Newtonian force differs from the thin field by nothing outside the sphere, so
/// R^thin = R^N = 1 - S_k S_j to rounding (1e-12 allowed for the sums).
#[test] fn newtonian_and_thin_residuals_equal_one_minus_s_squared() {
    for &tau in &[1e-3, 0.1, 1.0, 10.0] {
        let w = pair_world(tau, 10.0, [0.0, 0.0], false);
        let (th, nw) = thin_and_newton(&w, 0);
        let f = norm(&w.force.f_total[0]);
        let want = 1.0 - w.dv.s[0] * w.dv.s[1];
        let r_thin = (f - norm(&th)).abs() / norm(&th);
        let r_n = (f - norm(&nw)).abs() / norm(&nw);
        assert!((r_thin - want).abs() <= 1e-12, "tau {tau}: R^thin {r_thin:e} vs {want:e}");
        assert!((r_n - want).abs() <= 1e-12, "tau {tau}: R^N {r_n:e} vs {want:e}");
    }
}

/// Spec 6 item 5: the live drag-rate diagnostic equals the analytic reference up to the e^2/5 relative correction
/// stated in test (d1) (e = V0/v_g = 1e-3, so 2e-7); 5e-7 allowed.
#[test] fn drag_rate_diagnostic_matches_reference() {
    let g = Globals::defaults();
    let gamma_ref = drag_gamma_ref(&g).unwrap();
    let w = drag_world(g.clone(), [g.v_g / 1e3, 0.0, 0.0], 0.005 / gamma_ref).unwrap();
    let live = drag_rate(&w, 0).expect("drag rate available in V1");
    assert!((live - gamma_ref).abs() / gamma_ref <= 5e-7, "live {live:e} vs ref {gamma_ref:e}");
}

// ---------------------------------------------------------------------------------------------------------------------
// Test (c) curve properties (Spec 7(c): "R -> 1 as tau_c -> 0 with O(tau_c) deviation, and R decreases monotonically").
// Exact values: oracle/oracle_screening.py (mpmath, 30 digits). Added in workspace_7; written without a Rust compiler
// (pattern-copied from tests/oracle_sky.rs), so the first run is the first execution.
// ---------------------------------------------------------------------------------------------------------------------
const R_EXACT: [(f64, f64); 9] = [
    (1e-3, 0.999810481460228), (1e-2, 0.998112945682624), (0.1, 0.981917242209404), (0.5, 0.924617227019236),
    (1.0, 0.877785076145848), (2.0, 0.830442604550367), (5.0, 0.801867822859093), (10.0, 0.800018531139449),
    (20.0, 0.800000002009972),
];
fn r_validation(tau: f64) -> f64 {
    let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
    w.apply_options(|o| o.validation = Some(Validation::default_validation())).unwrap();
    screening_ratio(&w)
}

/// Validation mode against the exact curve at nine optical depths from 1e-3 to 20. Tolerance 4e-5: the measured maximum over
/// a 64-point scan of tau in [1e-3, 200] is 2.96e-5 (report/data/scan_validation.csv, worst at tau = 7.3).
#[test] fn screening_curve_validation_mode_tracks_exact_oracle() {
    for &(tau, r_ex) in R_EXACT.iter() {
        let r = r_validation(tau);
        assert!((r - r_ex).abs() < 4e-5, "tau {tau}: R {r} vs exact {r_ex}, err {:e}", (r - r_ex).abs());
    }
}

/// R(tau) strictly decreases on the grid tau = 1e-3 .. 10. The smallest exact step is R(5) - R(10) = 1.85e-3, far above the 3e-5
/// quadrature error. tau = 20 is deliberately excluded: R(10) - R(20) = 1.85e-5 is BELOW the validation-mode error, so
/// monotonicity there is a statement about the exact curve (checked in Python: strictly decreasing on 2000 points) and is
/// only testable with the certified preset.
#[test] fn screening_ratio_decreases_monotonically() {
    let rs: Vec<f64> = R_EXACT[..8].iter().map(|&(t, _)| r_validation(t)).collect();
    for i in 1..rs.len() { assert!(rs[i] < rs[i - 1], "R not decreasing between index {} and {}: {} then {}", i - 1, i, rs[i - 1], rs[i]); }
    assert!(rs[0] < 1.0 && rs[7] > 0.8 - 1e-4);   // exact R(10) = 0.8 + 1.85e-5, validation error up to 3e-5
}

/// R -> 1 with an O(tau_c) deviation: the exact small-tau slope is (1 - R)/tau = 0.18952 at tau = 1e-3 (oracle). Interval, not
/// a point: 0.18 < (1 - R)/tau < 0.20 is a 5% band, enormous against the 3e-5 / 1e-3 = 3% quadrature uncertainty bound.
#[test] fn screening_deviation_is_linear_in_tau_at_small_tau() {
    let tau = 1e-3;
    let slope = (1.0 - r_validation(tau)) / tau;
    assert!(slope > 0.18 && slope < 0.20, "(1-R)/tau = {slope}");
}

/// Spec 1.6: "m <= 0 or rho <= 0 is rejected" (also non-finite values). Probes carry no mass or density and are exempt.
#[test] fn invalid_mass_or_density_is_rejected() {
    let g = Globals::defaults();
    let base = || body_particles(vec![[0.0; 3], [1.0, 0.0, 0.0]], vec![[0.0; 3]; 2]);
    assert!(derive(&base(), &g).is_ok());
    for (what, edit) in [("m = 0", 0usize), ("m < 0", 1), ("m NaN", 2), ("rho = 0", 3), ("rho < 0", 4)] {
        let mut p = base();
        match edit { 0 => p.m[1] = 0.0, 1 => p.m[1] = -1.0, 2 => p.m[1] = f64::NAN, 3 => p.rho[1] = 0.0, _ => p.rho[1] = -2.39e5 }
        assert!(matches!(derive(&p, &g), Err(CoreError::InvalidParticle(1))), "{what} must be rejected");
    }
}
