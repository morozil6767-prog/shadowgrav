//! workspace_9 gaps: A7 (mid-run non-finite abort), E11 (reference error wired into the diagnostics), E7 (tau_max lower bound),
//! E9 (paired Seeliger read-out). NOT EXECUTED when written: the session that wrote this file had no Rust toolchain. Every bound
//! below is derived (stated next to it), not measured; a failure is information about the code or about the derivation.
use shadowgrav_core::diagnostics::{
    reference_diag, seeliger_check, sky_and_heating, snapshot, snapshot_with, tau_max,
    SnapshotOptions,
};
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::reference::reference_check;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn build(pos: &[V3], tau: f64, mode: Mode) -> World {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let xs: Vec<V3> = pos.iter().map(|p| scale(p, a)).collect();
    World::new(
        body_particles(xs.clone(), vec![[0.0; 3]; xs.len()]),
        g,
        default_options(mode),
        SimConfig::fixed(1e-4),
    )
    .unwrap()
}
const TWO: [V3; 2] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]];
const COLLINEAR: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]];
const PARTIAL: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]];
const PARTIAL_3D: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]];

/// Everything an aborted step must leave exactly as it was (Spec 1.6: the step is aborted, the state is not half-updated).
struct Before {
    x: Vec<V3>,
    time: f64,
    steps: u64,
    f_total: Vec<V3>,
    acc: Vec<V3>,
}
fn before(w: &World) -> Before {
    Before {
        x: w.p.x.clone(),
        time: w.time(),
        steps: w.step_count,
        f_total: w.force.f_total.clone(),
        acc: w.acc.clone(),
    }
}
fn assert_untouched(w: &World, b: &Before, what: &str) {
    assert_eq!(w.p.x, b.x, "{what}: positions not restored");
    assert_eq!(w.time(), b.time, "{what}: time advanced");
    assert_eq!(w.step_count, b.steps, "{what}: step counter advanced");
    assert_eq!(w.force.f_total, b.f_total, "{what}: stored force replaced");
    assert_eq!(w.acc, b.acc, "{what}: stored acceleration replaced");
}

// ------------------------------------------------------------------------------------------------ A7: mid-run abort
/// A NaN that appears AFTER construction (the case `non_finite_positions_are_rejected` cannot reach) aborts the step with
/// NonFiniteState and rolls back. The abort restores the pre-step state; it does not repair it, so the corrupted velocity is
/// still there afterwards, and a repaired world steps normally again.
#[test]
fn midrun_nan_velocity_aborts_the_step_and_rolls_back() {
    let mut w = build(&TWO, 1.0, Mode::V0);
    advance_by(&mut w, 1e-4).unwrap(); // a healthy step first: this is mid-run, not construction
    let b = before(&w);
    w.p.v[1] = [f64::NAN, 0.0, 0.0]; // corrupt one velocity between steps
    let r = advance_by(&mut w, 1e-4);
    // the validation layer (World::validate -> Particles::validate) pre-empts the integrator's own check: rejected, nothing touched
    assert!(
        matches!(r, Err(SimError::Core(CoreError::InvalidParticle(1)))),
        "expected Core(InvalidParticle(1)), got {r:?}"
    );
    assert_untouched(&w, &b, "NaN velocity");
    assert!(
        w.p.v[1][0].is_nan(),
        "rollback restores the pre-step (corrupted) state, it does not invent a value"
    );
    w.p.v[1] = [0.0; 3];
    advance_by(&mut w, 1e-4).expect("a repaired world must step again");
    assert_eq!(w.step_count, b.steps + 1);
}

/// An overflow (finite inputs, Inf output) is caught the same way: h * v exceeds the f64 range in the drift.
#[test]
fn midrun_overflow_in_the_drift_aborts_the_step() {
    let mut w = build(&TWO, 1.0, Mode::V0);
    let b = before(&w);
    w.p.v[0] = [1e308, 0.0, 0.0];
    let r = advance_by(&mut w, 1e10); // 1e10 * 1e308 = Inf
    assert!(
        matches!(r, Err(SimError::NonFiniteState { .. })),
        "expected NonFiniteState, got {r:?}"
    );
    assert_untouched(&w, &b, "drift overflow");
}

/// The other abort trigger: the FORCE evaluation returns CoreError::NonFinite (here Pi = Inf makes every Gamma_k infinite).
/// The drift of that step has already moved x, so this also proves the positions are restored after the force path fails.
#[test]
fn midrun_non_finite_force_aborts_the_step_and_rolls_back() {
    let mut w = build(&TWO, 1.0, Mode::V0);
    let b = before(&w);
    w.g.pi_override = Some(1e300);
    w.g.xi = 1e300; // each finite and valid; Gamma_k = xi*Pi*A/(4 pi) overflows to Inf in the force path
    let r = advance_by(&mut w, 1e-4);
    assert!(
        matches!(r, Err(SimError::Core(CoreError::NonFinite { .. }))),
        "expected Core(NonFinite), got {r:?}"
    );
    assert_untouched(&w, &b, "non-finite force");
    w.g.pi_override = None;
    w.g.xi = 1.0;
    advance_by(&mut w, 1e-4).expect("restored globals must step again");
}

/// The same non-finite force through a live global edit: rejected, the edit is rolled back, derived data untouched.
#[test]
fn non_finite_force_from_a_live_edit_is_rolled_back() {
    let mut w = build(&TWO, 1.0, Mode::V0);
    let (b, ver) = (before(&w), w.derived_version);
    let r = w.apply_globals(|g| {
        g.pi_override = Some(1e300);
        g.xi = 1e300;
    });
    assert!(
        matches!(r, Err(SimError::Core(CoreError::NonFinite { .. }))),
        "expected Core(NonFinite), got {r:?}"
    );
    assert!(w.g.pi_override.is_none(), "global edit must be rolled back");
    assert_eq!(w.derived_version, ver);
    assert_untouched(&w, &b, "live edit");
}

// ------------------------------------------------------------------------------------------------ E11: reference error in the diagnostics
/// The diagnostic is the engine-vs-reference error of reference_check, passed through unchanged; on the geometries the
/// reference-mode tests already bound (standard mode, partial and 3D overlap) it is at most 3e-4 of the largest force plus the
/// reference's own resolution estimate.
#[test]
fn reference_diag_reports_the_engine_error() {
    for (name, pos) in [("planar", &PARTIAL), ("3D", &PARTIAL_3D)] {
        let w = build(pos, 1.0, Mode::V0);
        let d = reference_diag(&w, 100, 200);
        let direct = reference_check(&w, 100, 200).unwrap();
        assert_eq!(
            d.max_rel_diff(),
            Some(direct.max_rel_diff),
            "{name}: wiring must not alter the number"
        );
        assert_eq!(d.reference_est_err(), Some(direct.reference_est_err));
        let (e, est) = (d.max_rel_diff().unwrap(), d.reference_est_err().unwrap());
        eprintln!(
            "[diag] {name}: engine vs reference {e:.3e}, reference's own estimate {est:.1e}; {}",
            d.summary()
        );
        assert!(e.is_finite() && est.is_finite() && e >= 0.0);
        // measured engine error (standard) <= 1.2e-4 of the largest force (tests/reference_mode.rs); the reference's own resolution adds to it
        assert!(
            e <= 3e-4 + est,
            "{name}: standard-mode engine vs reference {e:e} (reference estimate {est:e})"
        );
        assert_eq!(d.exceeds_reference_resolution(), Some(e > est));
    }
}

/// Out of scope is reported as "not checked", never as agreement: V1, a too-coarse grid.
#[test]
fn reference_diag_out_of_scope_is_not_a_pass() {
    let w = build(&COLLINEAR, 1.0, Mode::V1);
    let d = reference_diag(&w, 20, 40);
    assert!(d.outcome.is_err());
    assert_eq!(d.max_rel_diff(), None);
    assert_eq!(d.exceeds_reference_resolution(), None);
    assert!(d.summary().starts_with("not checked"), "{}", d.summary());
    assert!(d.summary().contains("V0 only"), "{}", d.summary());
    let v0 = build(&COLLINEAR, 1.0, Mode::V0);
    assert!(
        reference_diag(&v0, 1, 8).outcome.is_err(),
        "grid too small must be refused, not divide by zero"
    );
}

/// snapshot() is unchanged (nothing extra, no work); snapshot_with carries the opt-in results.
#[test]
fn snapshot_extras_are_opt_in() {
    let w = build(&PARTIAL, 1.0, Mode::V0);
    let plain = snapshot(&w);
    assert!(plain.reference.is_none() && plain.tau_max.is_none());
    let s = snapshot_with(
        &w,
        &SnapshotOptions {
            reference: Some((60, 120)),
            tau_max: true,
        },
    );
    assert!(s
        .reference
        .as_ref()
        .and_then(|r| r.max_rel_diff())
        .is_some());
    assert_eq!(s.tau_max.as_ref().map(|t| t.len()), Some(3));
    assert_eq!(s.step, plain.step);
}

// ------------------------------------------------------------------------------------------------ E7: tau_max lower bound
/// Exact cases. Centre ray through a sphere has chord 2a, so tau = beta * 2a = tau_c: a single exterior occluder gives its
/// tau_c; collinear occluders along one ray add. Collinear A-B-C (equal spheres): A sees B and C on one axis (2 tau_c), B sees
/// A and C on opposite sides (tau_c), C like A. Tolerance 1e-12 relative (chord b ~ 1e-15 a, so L = 2a to ~1e-30).
#[test]
fn tau_max_exact_for_isolated_and_collinear_occluders() {
    let w = build(&COLLINEAR, 5.0, Mode::V0);
    let tc = w.dv.tau_c[0];
    let t = tau_max(&w);
    assert_eq!(t.len(), 3);
    for (k, want) in [(0usize, 2.0 * tc), (1, tc), (2, 2.0 * tc)] {
        assert_eq!(t[k].k, k);
        assert!(
            (t[k].lower_bound / want - 1.0).abs() < 1e-12,
            "body {k}: {} vs {}",
            t[k].lower_bound,
            want
        );
        assert!(t[k].from_leaves.is_none(), "not instrumented: no leaf set");
        assert_eq!(t[k].samples, 2);
        assert!(!t[k].static_geometry_only);
    }
    let two = build(&TWO, 3.0, Mode::V0);
    for tm in tau_max(&two) {
        assert!((tm.lower_bound / two.dv.tau_c[0] - 1.0).abs() < 1e-12);
    }
}

/// Bounds that must hold for any geometry: tau_k(n) <= sum_j tau_c,j (every chord is at most 2a), and the centre set already
/// reaches the largest single tau_c (all pairs here are exterior). With leaf records the bound can only grow.
#[test]
fn tau_max_respects_its_bounds_and_leaves_only_raise_it() {
    for (name, pos) in [
        ("planar", &PARTIAL),
        ("3D", &PARTIAL_3D),
        ("collinear", &COLLINEAR),
    ] {
        let mut w = build(pos, 5.0, Mode::V0);
        let plain = tau_max(&w);
        w.apply_options(|o| o.instrumented = true).unwrap();
        let inst = tau_max(&w);
        let tc = w.dv.tau_c[0];
        let n = pos.len();
        for k in 0..n {
            let sum_bound = (n as f64 - 1.0) * tc;
            assert!(
                plain[k].lower_bound <= sum_bound * (1.0 + 1e-12),
                "{name} {k}: {} > {}",
                plain[k].lower_bound,
                sum_bound
            );
            assert!(
                plain[k].lower_bound >= tc * (1.0 - 1e-12),
                "{name} {k}: below a single occluder's tau_c"
            );
            assert!(
                inst[k].lower_bound >= plain[k].lower_bound,
                "{name} {k}: leaves must not lower the bound"
            );
            assert!(
                inst[k].lower_bound <= sum_bound * (1.0 + 1e-12),
                "{name} {k}: leaf sample exceeded the bound"
            );
            let fl = inst[k]
                .from_leaves
                .expect("instrumented world has leaf records");
            assert!(fl >= 0.0 && inst[k].lower_bound >= fl);
        }
        // the sky mean can never exceed the maximum (collinear: both tiny/ordered; planar/3D: overlap leaves are sampled)
        if let Some(h) = sky_and_heating(&w, 0) {
            assert!(
                h.tau_bar <= inst[0].lower_bound * (1.0 + 1e-9),
                "{name}: tau_bar {} > tau_max {}",
                h.tau_bar,
                inst[0].lower_bound
            );
        }
        eprintln!(
            "[diag] {name}: tau_max lower bounds {:?}, with leaves {:?}",
            plain.iter().map(|t| t.lower_bound).collect::<Vec<_>>(),
            inst.iter().map(|t| t.lower_bound).collect::<Vec<_>>()
        );
    }
}

// ------------------------------------------------------------------------------------------------ E9: paired Seeliger read-out
/// Thin occluder far from the target (a ~ 0.01, tau_c = 1e-6, d = 100 a ~ 1, alpha = 0.05: the parameters of
/// identities::seeliger_single_pair, 2e-3), but now on the live world, from two full force evaluations. The per-body ratio must
/// equal the per-pair ratio (same kernel, same arguments, one pair, no overlap) to rounding.
#[test]
fn seeliger_paired_run_matches_exp_minus_alpha_d() {
    let w = build(&[[0.0; 3], [100.0, 0.0, 0.0]], 1e-6, Mode::V0);
    let alpha = 0.05;
    let c = seeliger_check(&w, alpha).unwrap();
    assert_eq!(c.pairs.len(), 2);
    for p in &c.pairs {
        let want = (-alpha * p.d).exp();
        assert_eq!(p.expected, Some(mathx_exp(-alpha * p.d)));
        assert!(p.ratio < 1.0 && p.ratio > 0.0);
        assert!(
            (p.ratio / want - 1.0).abs() < 2e-3,
            "pair ({},{}): ratio {} vs exp(-alpha d) {}",
            p.k,
            p.j,
            p.ratio,
            want
        );
        assert!(p.rel_dev.unwrap() < 2e-3);
    }
    assert!(c.max_pair_dev.unwrap() < 2e-3);
    assert_eq!(c.body_ratio.len(), 2);
    for &(k, r) in &c.body_ratio {
        let pr = c.pairs.iter().find(|p| p.k == k).unwrap().ratio;
        assert!(
            (r / pr - 1.0).abs() < 1e-12,
            "body {k}: full-run ratio {r} vs pair-kernel ratio {pr}"
        );
    }
    assert_eq!(w.g.alpha_bg, 0.0, "the check must not modify the world");
}
fn mathx_exp(x: f64) -> f64 {
    shadowgrav_core::mathx::exp(x)
}

/// Refusals: alpha must be positive and finite; V2 has no alpha_bg > 0 (Spec gating).
#[test]
fn seeliger_check_refuses_bad_input() {
    let w = build(&TWO, 1.0, Mode::V0);
    for bad in [0.0, -0.1, f64::NAN, f64::INFINITY] {
        assert!(seeliger_check(&w, bad).is_err(), "{bad}");
    }
    let v2 = build(&TWO, 1.0, Mode::V2);
    assert!(matches!(
        seeliger_check(&v2, 0.01),
        Err(CoreError::Gating(_))
    ));
}

// ------------------------------------------------------------------------------------------------ audit follow-ups (2026-11-07)
/// Spec 6.12: the V2-valid flag was true in V0/V1 whatever the velocities; it is now a property of the velocities in every mode.
#[test]
fn v2_valid_flag_reports_fast_bodies_in_every_mode() {
    for mode in [Mode::V0, Mode::V1] {
        let mut w = build(&TWO, 1.0, mode);
        assert!(w.force.status.v2_valid, "at rest");
        w.set_velocities(vec![[0.5 * w.g.v_g, 0.0, 0.0], [0.0; 3]])
            .unwrap(); // |eps| = 0.5 > 0.1
        w.refresh_forces().unwrap();
        assert!(
            !w.force.status.v2_valid,
            "{mode:?}: |eps| = 0.5 must clear the flag"
        );
        w.set_velocities(vec![[0.05 * w.g.v_g, 0.0, 0.0], [0.0; 3]])
            .unwrap();
        w.refresh_forces().unwrap();
        assert!(
            w.force.status.v2_valid,
            "{mode:?}: |eps| = 0.05 is admissible"
        );
    }
}
#[test]
fn aggregates_match_per_body_values_and_distinguish_not_measured() {
    use shadowgrav_core::diagnostics::aggregates;
    let w = build(&COLLINEAR, 5.0, Mode::V0);
    let a = aggregates(&w);
    let t = a.tau_c.unwrap();
    assert!(
        (t.min - 5.0).abs() < 1e-12 && (t.max - 5.0).abs() < 1e-12 && (t.mean - 5.0).abs() < 1e-12
    );
    let s = a.s.unwrap();
    assert!(s.min > 0.0 && s.max <= 1.0 && s.min <= s.mean && s.mean <= s.max);
    assert!(
        a.d_bar.is_none() && a.p_heat_total.is_none(),
        "not instrumented: NOT MEASURED, never 0"
    );
}
#[test]
fn total_heating_power_is_the_sum_of_per_body_powers() {
    use shadowgrav_core::diagnostics::aggregates;
    let mut w = build(&COLLINEAR, 1.0, Mode::V0);
    w.apply_options(|o| o.instrumented = true).unwrap();
    let a = aggregates(&w);
    let sum: f64 = (0..3).map(|k| sky_and_heating(&w, k).unwrap().p_heat).sum();
    let tot = a.p_heat_total.expect("instrumented world");
    assert!((tot - sum).abs() <= 1e-12 * sum.abs(), "{tot:e} vs {sum:e}");
    let d = a.d_bar.unwrap();
    assert!(d.min <= d.mean && d.mean <= d.max && d.max <= 1.0 && d.min >= 0.0);
}
#[test]
fn out_of_table_parameters_produce_warnings() {
    use shadowgrav_core::diagnostics::spec_range_warnings;
    let mut w = build(&TWO, 1.0, Mode::V0);
    assert!(
        spec_range_warnings(&w).is_empty(),
        "defaults are inside the Spec table"
    );
    w.apply_globals(|g| {
        g.xi = 3.0;
        g.f = 0.9;
    })
    .unwrap();
    let v = spec_range_warnings(&w);
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(v.iter().any(|s| s.contains("xi")) && v.iter().any(|s| s.starts_with("f ")));
}
