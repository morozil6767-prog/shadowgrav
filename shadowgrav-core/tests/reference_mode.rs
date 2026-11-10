//! Spec 4.4 reference mode (all-rays, O(N^3)), Traceability D5/E11. The reference is itself validated against the
//! exact axisymmetric oracle (test (c)), then used to judge the production quadrature on geometries with no closed form.
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::reference::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn world(tau: f64, pos: &[V3], mode: Mode) -> World {
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
const COLLINEAR: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]];
const PARTIAL: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]];
const PARTIAL_3D: [V3; 3] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]];

/// The reference reproduces the EXACT screening ratio R(5) = 0.801867822859093 (independent 30-digit oracle),
/// with R built from the reference force on A over the engine's exact closed-form pair sum |F_AB| + |F_AC|.
/// Bound: the reference's own resolution estimate (the difference between grids n and 2n, normalised by the largest force);
/// measured 3.5e-5 against an estimate of 1.3e-4 (the concentric silhouette of C inside B converges first order).
#[test]
fn reference_reproduces_exact_screening_ratio() {
    let w = world(5.0, &COLLINEAR, Mode::V0);
    let r = reference_forces(&w.p, &w.dv, &w.g, Mode::V0, 200, 400).unwrap();
    let pair = norm(&w.force.f_pair[0]);
    let ratio = norm(&r.f[0]) / pair;
    let err = (ratio - 0.801867822859093).abs();
    eprintln!(
        "[reference] R_ref = {ratio:.9}, |R_ref - exact| = {err:.2e}, own estimate {:.2e}",
        r.est_rel_err
    );
    assert!(
        err <= r.est_rel_err,
        "err {err:e} vs own estimate {:e}",
        r.est_rel_err
    );
    // symmetric geometry: F_B = 0 and F_A = -F_C
    assert!(norm(&r.f[1]) < 1e-6 * norm(&r.f[0]), "F_B = {:?}", r.f[1]);
    assert!(norm(&add(&r.f[0], &r.f[2])) < 1e-6 * norm(&r.f[0]));
}

/// Refining the reference converges (est error falls) and the engine in validation mode agrees with it to the Spec 4.3
/// target 1e-4 on partial and 3D overlap. Errors are relative to the LARGEST force in the system (a body whose force is small
/// would otherwise dominate: measured per-body-relative standard-mode error reaches 9.6e-4, relative to the largest force <= 1.2e-4).
#[test]
fn reference_judges_the_production_quadrature() {
    for (name, pos) in [("planar", &PARTIAL), ("3D", &PARTIAL_3D)] {
        for tau in [1.0, 5.0] {
            let mut w = world(tau, pos, Mode::V0);
            let coarse = reference_check(&w, 100, 200).unwrap();
            let fine = reference_check(&w, 200, 400).unwrap();
            assert!(
                fine.reference_est_err <= coarse.reference_est_err + 1e-12,
                "{name} tau {tau}: reference not converging"
            );
            let std = fine.max_rel_diff;
            w.apply_options(|o| o.validation = Some(Validation::default_validation()))
                .unwrap();
            let val = reference_check(&w, 200, 400).unwrap();
            eprintln!("[reference] {name:6} tau {tau}: ref est {:.1e}; engine standard {std:.2e}, validation {:.2e}", fine.reference_est_err, val.max_rel_diff);
            assert!(
                val.max_rel_diff <= 1e-4 + val.reference_est_err,
                "{name} tau {tau}: validation vs reference {:e}",
                val.max_rel_diff
            );
            assert!(
                std <= 3e-4,
                "{name} tau {tau}: standard vs reference {std:e}"
            );
        }
    }
}

/// Out-of-scope inputs are refused, never answered wrongly.
#[test]
fn reference_refuses_out_of_scope_worlds() {
    let w = world(1.0, &COLLINEAR, Mode::V0);
    assert!(
        reference_forces(&w.p, &w.dv, &w.g, Mode::V1, 20, 40).is_err(),
        "V1"
    );
    let mut g = w.g.clone();
    g.alpha_bg = 1e-3;
    assert!(
        reference_forces(&w.p, &w.dv, &g, Mode::V0, 20, 40).is_err(),
        "alpha_bg > 0"
    );
    let a = sphere_radius(1.0, RHO0);
    let inside = world(1.0, &[[0.0; 3], [0.5, 0.0, 0.0]], Mode::V0); // 0.5 a apart: interior pair
    assert!(
        reference_forces(&inside.p, &inside.dv, &inside.g, Mode::V0, 20, 40).is_err(),
        "interior pair (a = {a})"
    );
    let big: Vec<V3> = (0..51).map(|i| [10.0 * i as f64, 0.0, 0.0]).collect();
    let wbig = world(1e-3, &big, Mode::V0);
    assert!(
        reference_forces(&wbig.p, &wbig.dv, &wbig.g, Mode::V0, 4, 8).is_err(),
        "N > 50"
    );
}

/// A single body feels nothing; two well separated bodies reproduce the closed-form pair force (disjoint discs, no
/// overlap correction) to the reference's own resolution.
#[test]
fn reference_two_body_matches_closed_form_pair() {
    let w = world(1.0, &[[0.0; 3], [10.0, 0.0, 0.0]], Mode::V0);
    let r = reference_forces(&w.p, &w.dv, &w.g, Mode::V0, 200, 40).unwrap();
    for k in 0..2 {
        let rel = norm(&sub(&r.f[k], &w.force.f_total[k])) / norm(&w.force.f_total[k]);
        eprintln!(
            "[reference] two-body body {k}: rel diff {rel:.3e}, est {:.2e}",
            r.est_rel_err
        );
        assert!(
            rel <= 2.0 * r.est_rel_err,
            "body {k}: |F_ref - F_engine|/|F| = {rel:e}"
        ); // measured 1.7e-6 vs estimate 5.2e-6
    }
    let one = world(1.0, &[[0.0; 3]], Mode::V0);
    let r1 = reference_forces(&one.p, &one.dv, &one.g, Mode::V0, 10, 10).unwrap();
    assert_eq!(r1.f[0], [0.0; 3]);
}
