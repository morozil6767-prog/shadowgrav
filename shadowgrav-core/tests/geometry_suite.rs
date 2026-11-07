//! Broader geometry battery: multi-config regression for the production force path.
use shadowgrav_core::diagnostics::{conservation, delta_na, pair_effective_g};
use shadowgrav_core::driver::{
    default_options, run_orbit, screening_ratio, three_body_collinear, two_body_orbit,
};
use shadowgrav_core::geom::norm;
use shadowgrav_core::state::{SimConfig, World};
use shadowgrav_core::driver::RHO0;
use shadowgrav_core::{advance, Kind, Mode, Options, Particles, Validation};

fn opt_val() -> Options {
    let mut o = default_options(Mode::V0);
    o.validation = Some(Validation::default_validation());
    o.parallel = false;
    o
}

fn world_from_positions(xs: Vec<[f64; 3]>, mut opt: Options) -> World {
    let n = xs.len();
    let p = Particles {
        x: xs,
        v: vec![[0.0; 3]; n],
        m: vec![1.0; n],
        rho: vec![RHO0; n],
        delta: vec![0.0; n],
        kind: vec![Kind::Body; n],
    };
    opt.instrumented = true;
    World::new(p, shadowgrav_core::Globals::defaults(), opt, SimConfig::fixed(1e-3)).expect("world")
}

#[test]
fn collinear_screening_standard_within_provisional() {
    let w = three_body_collinear(20.0, 1e-4, 0).expect("collinear");
    let r = screening_ratio(&w);
    assert!(r.is_finite(), "R not finite");
    let err = (r - 0.8).abs();
    assert!(err <= 5e-3, "standard |R-0.8|={err}");
}

#[test]
fn collinear_screening_validation_tighter_than_standard() {
    let mut w = three_body_collinear(20.0, 1e-4, 0).unwrap();
    let err_std = (screening_ratio(&w) - 0.8).abs();
    w.apply_options(|o| o.validation = Some(Validation::default_validation())).unwrap();
    let err_val = (screening_ratio(&w) - 0.8).abs();
    assert!(err_val < err_std, "validation {err_val} should beat standard {err_std}");
    assert!(err_val < 5e-5, "validation |R-0.8|={err_val}");
}

#[test]
fn partial_and_3d_layouts_finite_forces_and_nonnegative_delta_na() {
    let layouts = [
        vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]],
        vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]],
        vec![[0.0, 0.0, 0.0], [8.0, 0.0, 0.0], [6.0, 5.0, 0.0], [4.0, -4.0, 2.0]],
    ];
    for xs in layouts {
        let w = world_from_positions(xs, opt_val());
        for i in 0..w.n() {
            assert!(norm(&w.force.f_total[i]).is_finite());
            assert!(norm(&w.force.f_corr[i]).is_finite());
            let na = delta_na(&w, i);
            assert!(na.is_finite() && na >= -1e-12, "delta_na[{i}]={na}");
        }
    }
}

#[test]
fn n12_ring_momentum_residual_under_validation() {
    let mut xs = Vec::new();
    for i in 0..12 {
        let t = std::f64::consts::TAU * i as f64 / 12.0;
        xs.push([5.0 * t.cos(), 5.0 * t.sin(), 0.15 * (i as f64 % 3.0)]);
    }
    let w = world_from_positions(xs, opt_val());
    let mut sum = [0.0; 3];
    let mut abs = 0.0;
    for i in 0..w.n() {
        for c in 0..3 {
            sum[c] += w.force.f_total[i][c];
        }
        abs += norm(&w.force.f_total[i]);
    }
    let rp = norm(&sum) / abs.max(1e-300);
    assert!(rp < 1e-6, "R_P={rp}");
}

#[test]
fn geff_matches_expected_when_disjoint() {
    let xs = vec![[-2.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let mut o = default_options(Mode::V0);
    o.parallel = false;
    o.instrumented = true;
    let w = world_from_positions(xs, o);
    let pairs = pair_effective_g(&w).expect("instrumented sinks");
    assert!(!pairs.is_empty());
    for pg in pairs {
        if let Some(exp) = pg.expected {
            let rel = (pg.g_eff - exp).abs() / exp.abs().max(1e-300);
            assert!(rel < 1e-9, "G_eff {} vs expected {} rel={rel}", pg.g_eff, exp);
        }
    }
}

#[test]
fn twenty_orbit_energy_and_angular_momentum_bounded() {
    let setup = two_body_orbit(1e-3, 400, Mode::V0, 1e3).expect("orbit setup");
    let report = run_orbit(setup, 20).expect("20 orbits");
    assert!(report.max_de < 1e-4, "max dE/E={} over 20 orbits", report.max_de);
    assert!(report.max_dl < 1e-10, "max dL/L={} over 20 orbits", report.max_dl);
}

#[test]
fn advance_100_steps_n7_no_nonfinite() {
    let mut xs = Vec::new();
    for i in 0..7 {
        let t = 0.9 * i as f64;
        xs.push([3.5 * t.cos(), 3.5 * t.sin(), 0.2 * t.sin()]);
    }
    let mut w = world_from_positions(xs, opt_val());
    let vs: Vec<_> = (0..w.n()).map(|i| [0.01 * i as f64, 0.0, 0.0]).collect();
    w.set_velocities(vs).unwrap();
    for _ in 0..100 {
        advance(&mut w).expect("step");
    }
    for i in 0..w.n() {
        for c in 0..3 {
            assert!(w.p.x[i][c].is_finite());
            assert!(w.p.v[i][c].is_finite());
            assert!(w.force.f_total[i][c].is_finite());
        }
    }
    let c = conservation(&w);
    assert!(c.e.is_finite() && c.l.iter().all(|x| x.is_finite()));
}

#[test]
fn two_body_force_is_translation_and_rotation_equivariant() {
    use shadowgrav_core::geom::{add, dot, scale, sub};
    use shadowgrav_core::{Particles, Globals};
    let base = [0.7, -1.1, 0.4];
    let offset = [2.5, -3.0, 0.75];
    let shifted = add(&base, &offset);
    let make = |x0: [f64; 3], x1: [f64; 3]| {
        let mut o = default_options(Mode::V0);
        o.parallel = false;
        World::new(
            Particles {
                x: vec![x0, x1], v: vec![[0.0; 3]; 2], m: vec![1.0; 2],
                rho: vec![RHO0; 2], delta: vec![0.0; 2], kind: vec![Kind::Body; 2],
            },
            Globals::defaults(), o, SimConfig::fixed(1e-3),
        ).expect("valid pair")
    };
    let origin = make([0.0; 3], base);
    let translated = make(offset, shifted);
    let rotate_z = |v: [f64; 3]| [-v[1], v[0], v[2]];
    let rotated = make([0.0; 3], rotate_z(base));
    let f = origin.force.f_total[0];
    let ft = translated.force.f_total[0];
    let fr = rotated.force.f_total[0];
    let close = |a: [f64; 3], b: [f64; 3]| {
        let d = sub(&a, &b);
        norm(&d) / norm(&b).max(1e-300)
    };
    assert!(close(f, ft) < 2e-13, "translation changed pair force: {f:?} vs {ft:?}");
    assert!(close(rotate_z(f), fr) < 2e-13, "rotation changed pair force: {:?} vs {fr:?}", rotate_z(f));
    let c = scale(&base, 1.0 / norm(&base));
    assert!(norm(&[f[1] * c[2] - f[2] * c[1], f[2] * c[0] - f[0] * c[2], f[0] * c[1] - f[1] * c[0]]) < 1e-12 * norm(&f));
    assert!(dot(&f, &c) > 0.0, "force must point toward the occluder");
    assert!(norm(&add(&origin.force.f_total[0], &origin.force.f_total[1])) < 1e-12 * norm(&f));
}

#[test]
fn collimated_nonoverlapping_bodies_exercise_nonadditive_stress_path() {
    let a = shadowgrav_core::driver::sphere_radius(1.0, RHO0);
    let mut xs = vec![[0.0; 3]];
    // Six well-separated spheres on nearly the same line create a multi-disc overlap
    // on target 0 without overlapping the physical bodies themselves.
    for i in 0..6 {
        let r = (100.0 + 10.0 * i as f64) * a;
        let side = (i as f64 - 2.5) * 0.08 * a;
        xs.push([side, -0.5 * side, r]);
    }
    let mut g = shadowgrav_core::Globals::defaults();
    g.kappa0 = shadowgrav_core::driver::kappa0_for_tau(0.5, 1.0, RHO0);
    let n = xs.len();
    let p = Particles {
        x: xs, v: vec![[0.0; 3]; n], m: vec![1.0; n], rho: vec![RHO0; n],
        delta: vec![0.0; n], kind: vec![Kind::Body; n],
    };
    let mut o = opt_val();
    o.instrumented = false;
    let w = World::new(p, g, o, SimConfig::fixed(1e-4)).expect("stress geometry");
    assert!(w.force.status.overlap_present, "the engineered angular discs must overlap");
    for (i, f) in w.force.f_total.iter().enumerate() {
        assert!(f.iter().all(|v| v.is_finite()), "non-finite force on body {i}: {f:?}");
    }
    let na = delta_na(&w, 0);
    assert!(na.is_finite() && na > 0.0, "overlap correction should be active; Delta^NA={na}");
}
