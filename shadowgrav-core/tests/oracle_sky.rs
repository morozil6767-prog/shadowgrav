//! Cross-check against an INDEPENDENT brute-force sky quadrature (oracle/oracle_sky.py: Fibonacci grid, 1.6e7 rays,
//! no HEALPix / overlap graph / closed forms). Configurations have PARTIAL (non-nested) disc overlap and a genuinely
//! 3D arrangement, so they exercise the general candidate graph and touching-set machinery that the collinear test (c)
//! cannot. Reference vectors are F_k / Gamma_k = int n D_k dOmega (dimensionless); oracle noise ~6e-5 (N^-3/4).
use shadowgrav_core::diagnostics::delta_na;
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;
use std::f64::consts::PI;

struct Case {
    x: [[f64; 3]; 3],
    tau: f64,
    f0: [[f64; 3]; 3],
    na0: f64,
}

const CASES: [Case; 4] = [
    Case {
        x: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]],
        tau: 1.0,
        na0: 0.00287756191443983,
        f0: [
            [2.4615191860e-02, 1.6389562854e-03, 4.3861318117e-09],
            [1.1611800213e-01, 1.3093135112e-01, -4.9413168246e-08],
            [-1.4080396036e-01, -1.3257665783e-01, 2.9594894041e-08],
        ],
    },
    Case {
        x: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]],
        tau: 5.0,
        na0: 0.011292426795888624,
        f0: [
            [4.7796430252e-02, 3.1722996761e-03, 3.1487692497e-08],
            [2.2735975606e-01, 2.5636485772e-01, -2.4756652920e-07],
            [-2.7569518570e-01, -2.5958597092e-01, 1.5292327213e-07],
        ],
    },
    Case {
        x: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]],
        tau: 1.0,
        na0: 0.003550047035907218,
        f0: [
            [2.4606709945e-02, 1.1471449630e-03, 1.1471153478e-03],
            [1.1810661968e-01, 9.3043889186e-02, 9.3043819268e-02],
            [-1.4280082645e-01, -9.4196654797e-02, -9.4196759177e-02],
        ],
    },
    Case {
        x: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]],
        tau: 5.0,
        na0: 0.013393782411369088,
        f0: [
            [4.7712978011e-02, 2.2160474118e-03, 2.2159447207e-03],
            [2.3125296538e-01, 1.8218018931e-01, 1.8217986063e-01],
            [-2.7960507237e-01, -1.8443738754e-01, -1.8443786923e-01],
        ],
    },
];

/// Returns (max over bodies of |dF|/|F| against the oracle vectors, relative error of Delta^NA of body 0).
/// Always prints its measurements (visible with `-- --nocapture`), so the tolerances below can be audited.
fn measure(c: &Case, val: Option<Validation>, label: &str) -> (f64, f64) {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(c.tau, 1.0, RHO0);
    let xs: Vec<V3> = c.x.iter().map(|p| scale(p, a)).collect();
    let mut opt = default_options(Mode::V0);
    opt.validation = val;
    let w = World::new(
        body_particles(xs, vec![[0.0; 3]; 3]),
        g,
        opt,
        SimConfig::fixed(1e-4),
    )
    .unwrap();
    assert!(
        w.force.status.overlap_present,
        "{label}: overlap must be detected"
    );
    let mut worst = 0.0f64;
    for k in 0..3 {
        let gam = w.g.xi * w.g.pi_flux() * w.dv.a_eff[k] / (4.0 * PI);
        let got = scale(&w.force.f_total[k], 1.0 / gam);
        worst = worst.max(norm(&sub(&got, &c.f0[k])) / norm(&c.f0[k]));
    }
    // the non-additive correction itself must be reproduced, not just the total (which is dominated by pair terms)
    let na = delta_na(&w, 0);
    let na_rel = (na - c.na0).abs() / c.na0;
    eprintln!("[oracle_sky] {label:10} tau {} x2={:?}: max|dF|/|F| = {worst:.2e}, Delta^NA = {na:.6e} (oracle {:.6e}, rel err {na_rel:.2e}), leaves {}",
              c.tau, c.x[2], c.na0, w.force.status.leaf_count);
    (worst, na_rel)
}

/// Measured (CONVERGENCE.md, section 3): standard mode (f = 0.15, ~55 leaves) is a coarse quadrature on partially
/// overlapping discs: max |dF|/|F| = 9.6e-4 and Delta^NA off by up to 8.6% (largest at tau = 5). These bounds are
/// REGRESSION bounds (about 1.5x the measurement), not accuracy claims; the honest accuracy statement is the measurement.
#[test]
fn partial_and_3d_overlap_standard_mode_matches_sky_oracle() {
    for c in CASES.iter() {
        let (v, n) = measure(c, None, "standard");
        assert!(v <= 1.5e-3, "standard: |dF|/|F| = {v:e}");
        assert!(n <= 0.13, "standard: Delta^NA relative error {n:e}");
    }
}
/// Validation mode: measured max |dF|/|F| = 3.7e-5 (at the oracle's own noise level, ~6e-5), Delta^NA within 0.42%.
#[test]
fn partial_and_3d_overlap_validation_mode_matches_sky_oracle() {
    for c in CASES.iter() {
        let (v, n) = measure(c, Some(Validation::default_validation()), "validation");
        assert!(v <= 1e-4, "validation: |dF|/|F| = {v:e}");
        assert!(n <= 1e-2, "validation: Delta^NA relative error {n:e}");
    }
}

/// Exact R(tau) for test (c) from oracle/oracle_screening.py (1D axisymmetric integral, mpmath 30 digits).
/// Spec expects R -> 0.8 as tau -> inf; the oracle gives R(20) = 0.800000002 (so the 1e-6 requirement is physical, not a
/// convention), and the whole curve below is monotone decreasing.
#[test]
fn screening_curve_matches_exact_oracle() {
    let exact = [
        (1.0, 0.877785076145848),
        (5.0, 0.801867822859093),
        (20.0, 0.800000002009972),
    ];
    for &(tau, r_ex) in &exact {
        let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
        let r_std = screening_ratio(&w);
        assert!(
            (r_std - r_ex).abs() < 5e-3,
            "standard tau {tau}: R {r_std} vs {r_ex}"
        );
        w.apply_options(|o| o.validation = Some(Validation::default_validation()))
            .unwrap();
        let r_val = screening_ratio(&w);
        assert!(
            (r_val - r_ex).abs() < 4e-5,
            "validation tau {tau}: R {r_val} vs {r_ex}, err {:e}",
            (r_val - r_ex).abs()
        );
    }
}

/// Certified preset against the exact curve: |R - R_exact| < 1e-6 at tau = 1, 5, 20 (measured 4.7e-8, 1.8e-7, 4.3e-7).
/// ~40 s in total, so ignored by default: `cargo test --release -p shadowgrav-core --test oracle_sky -- --ignored`.
#[test]
#[ignore = "~40 s: three 65M-leaf evaluations"]
fn screening_curve_certified_mode_matches_exact_oracle() {
    let exact = [
        (1.0, 0.877785076145848),
        (5.0, 0.801867822859093),
        (20.0, 0.800000002009972),
    ];
    for &(tau, r_ex) in &exact {
        let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
        w.apply_options(|o| o.validation = Some(Validation::certified()))
            .unwrap();
        let r = screening_ratio(&w);
        assert!(
            (r - r_ex).abs() < 1e-6,
            "certified tau {tau}: R {r} vs {r_ex}, err {:e}",
            (r - r_ex).abs()
        );
    }
}
