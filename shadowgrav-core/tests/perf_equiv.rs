//! Performance-phase equivalence tests: the speed-ups must not change a single bit of the conforming result.
//!   * spatial-hash candidate graph == O(m^2) reference graph (static and V2 shifted discs, with big and tiny discs)
//!   * rayon-parallel force == serial force == brute-candidate force (bitwise)
//!   * thin-fast preview: gated, flagged, and within Barnes-Hut error of the conforming engine
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::overlap::*;
use shadowgrav_core::*;
use std::f64::consts::PI;

struct Rng(u64);
impl Rng {
    fn u(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Mirrors the occluder construction in force.rs (static + I.4 shifted disc).
fn make_occ(c: V3, a: f64, eps: V3, v2: bool) -> Occ {
    let d = norm(&c);
    let chat = scale(&c, 1.0 / d);
    let theta = if d > a {
        a.atan2(((d - a) * (d + a)).sqrt())
    } else {
        PI
    };
    let e = norm(&eps);
    let (sh_c, sh_theta) = if v2 {
        let qe = dot(&chat, &eps);
        let w = (qe * qe + 1.0 - e * e).sqrt();
        let lam = qe + w;
        (
            normalize(&sub(&scale(&chat, lam), &eps)),
            (theta / (1.0 - e)).min(PI),
        )
    } else {
        (chat, theta)
    };
    Occ {
        c,
        chat,
        d,
        a,
        beta: 1.0,
        theta,
        theta_eff: theta.min(1.0),
        eps,
        e,
        sh_c,
        sh_theta,
    }
}

#[test]
fn hash_candidates_equal_brute() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut total = 0usize;
    for trial in 0..40 {
        let v2 = trial % 2 == 1;
        let m = 300;
        let mut occ = Vec::new();
        for i in 0..m {
            let c = loop {
                let c = [rng.u() - 0.5, rng.u() - 0.5, rng.u() - 0.5];
                if norm(&c) > 0.03 {
                    break c;
                }
            };
            let a = if trial % 3 == 0 && i < 3 {
                0.15
            } else if i % 50 == 7 {
                1e-5
            } else {
                0.01
            }; // big and tiny discs
            let eps = if v2 {
                [
                    0.05 * (rng.u() - 0.5),
                    0.05 * (rng.u() - 0.5),
                    0.05 * (rng.u() - 0.5),
                ]
            } else {
                [0.0; 3]
            };
            occ.push(make_occ(c, a, eps, v2));
        }
        let (b, h) = (candidates_brute(&occ, v2), candidates_with(&occ, v2, false));
        assert_eq!(b, h, "trial {trial} v2 {v2}");
        total += b.len();
    }
    assert!(total > 0, "test would be vacuous: no overlaps generated");
}

fn config(n: usize, side: f64, tau: f64, seed: u64) -> (Particles, Globals) {
    let a = sphere_radius(1.0, RHO0);
    let mut rng = Rng(seed);
    let mut xs: Vec<V3> = Vec::new();
    while xs.len() < n {
        let c = [side * rng.u(), side * rng.u(), side * rng.u()];
        if xs.iter().all(|x| norm(&sub(x, &c)) > 2.0 * a) {
            xs.push(c);
        }
    }
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    (body_particles(xs, vec![[0.0; 3]; n]), g)
}

fn forces(p: &Particles, g: &Globals, parallel: bool, brute: bool) -> ForceOutput {
    let dv = derive(p, g).unwrap();
    let mut o = default_options(Mode::V0);
    o.parallel = parallel;
    o.brute_candidates = brute;
    evaluate_forces(p, &dv, g, &o).unwrap()
}

#[test]
fn parallel_hash_and_brute_are_bitwise_identical() {
    let (p, g) = config(70, 1.0, 0.5, 12345);
    let base = forces(&p, &g, false, true);
    assert!(base.status.overlap_present, "config must contain overlaps");
    for &(par, brute) in &[(false, false), (true, true), (true, false)] {
        let f = forces(&p, &g, par, brute);
        assert_eq!(f.f_total, base.f_total, "parallel {par} brute {brute}");
        assert_eq!(f.f_corr, base.f_corr);
        assert_eq!(f.status.leaf_count, base.status.leaf_count);
    }
}

#[test]
fn thin_fast_matches_conforming_within_barnes_hut_error() {
    let (p, g) = config(150, 3.0, 1e-6, 777);
    let dv = derive(&p, &g).unwrap();
    let exact = evaluate_forces(&p, &dv, &g, &default_options(Mode::V0)).unwrap();
    let mut o = default_options(Mode::V0);
    o.thin_fast = true;
    o.thin_theta = 0.5;
    let fast = evaluate_forces(&p, &dv, &g, &o).unwrap();
    assert!(fast.status.thin_fast && !exact.status.thin_fast);
    let (mut e2, mut f2) = (0.0, 0.0);
    for k in 0..p.x.len() {
        let d = sub(&fast.f_total[k], &exact.f_total[k]);
        e2 += dot(&d, &d);
        f2 += dot(&exact.f_total[k], &exact.f_total[k]);
    }
    let rel = (e2 / f2).sqrt();
    assert!(
        rel < 2e-2,
        "Barnes-Hut (theta 0.5) vector-norm error {rel:e}"
    ); // Python model: ~5e-3
    o.thin_theta = 0.2;
    let tight = evaluate_forces(&p, &dv, &g, &o).unwrap();
    let (mut e3, mut f3) = (0.0, 0.0);
    for k in 0..p.x.len() {
        let d = sub(&tight.f_total[k], &exact.f_total[k]);
        e3 += dot(&d, &d);
        f3 += dot(&exact.f_total[k], &exact.f_total[k]);
    }
    assert!(
        (e3 / f3).sqrt() < rel,
        "smaller opening angle must be more accurate"
    );
}

#[test]
fn thin_fast_is_gated() {
    let mut o = default_options(Mode::V0);
    o.thin_fast = true;
    let (p, g) = config(10, 1.0, 1.0, 5); // tau_c = 1 is not thin
    let dv = derive(&p, &g).unwrap();
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &o),
        Err(CoreError::Gating(_))
    ));
    let (p, g) = config(10, 1.0, 1e-6, 5);
    let dv = derive(&p, &g).unwrap();
    o.mode = Mode::V1;
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &o),
        Err(CoreError::Gating(_))
    ));
    o.mode = Mode::V0;
    o.thin_theta = 0.9;
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &o),
        Err(CoreError::Config(_))
    ));
    o.thin_theta = 0.5;
    assert!(evaluate_forces(&p, &dv, &g, &o).is_ok());
}
