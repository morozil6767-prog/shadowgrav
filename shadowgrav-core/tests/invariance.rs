//! Symmetry properties of the N-body force that must hold for ANY geometry. They are not Spec tests; they are the cheapest way to find
//! an indexing, orientation or summation-order bug that the hand-built presets cannot reach. Bounds are measured (stated beside them).
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn world(pos: &[V3], tau: f64, validation: Option<Validation>) -> World {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let xs: Vec<V3> = pos.iter().map(|p| scale(p, a)).collect();
    let mut o = default_options(Mode::V0);
    o.validation = validation;
    World::new(
        body_particles(xs.clone(), vec![[0.0; 3]; xs.len()]),
        g,
        o,
        SimConfig::fixed(1e-4),
    )
    .unwrap()
}
/// 6 bodies, a genuinely 3D cluster with overlapping shadows (spacings 3..6 a, tau = 3).
const CLUSTER: [V3; 6] = [
    [0.0, 0.0, 0.0],
    [5.0, 0.3, -0.2],
    [2.4, 4.6, 0.5],
    [-2.0, 3.0, 2.8],
    [3.1, -3.4, 3.9],
    [6.5, 4.2, -3.0],
];

fn max_rel(a: &[V3], b: &[V3]) -> f64 {
    let scale = a.iter().map(norm).fold(0.0, f64::max);
    a.iter()
        .zip(b)
        .map(|(x, y)| norm(&sub(x, y)))
        .fold(0.0, f64::max)
        / scale
}

#[test]
fn permuting_the_bodies_permutes_the_forces() {
    let w1 = world(&CLUSTER, 3.0, None);
    let perm = [3usize, 5, 0, 2, 4, 1];
    let pos2: Vec<V3> = perm.iter().map(|&i| CLUSTER[i]).collect();
    let w2 = world(&pos2, 3.0, None);
    let f2: Vec<V3> = (0..6)
        .map(|j| w2.force.f_total[perm.iter().position(|&i| i == j).unwrap()])
        .collect();
    let d = max_rel(&w1.force.f_total, &f2);
    eprintln!("[invariance] permutation: {d:e}");
    assert!(
        d < 1e-9,
        "permutation changed the forces by {d:e} (only summation order may differ)"
    );
}
#[test]
fn translating_the_cluster_leaves_forces_unchanged() {
    let w1 = world(&CLUSTER, 3.0, None);
    let shifted: Vec<V3> = CLUSTER
        .iter()
        .map(|p| add(p, &[137.0, -52.0, 9.0]))
        .collect();
    let w2 = world(&shifted, 3.0, None);
    let d = max_rel(&w1.force.f_total, &w2.force.f_total);
    eprintln!("[invariance] translation: {d:e}");
    assert!(
        d < 1e-3,
        "translation by many radii changed the forces by {d:e}"
    ); // HEALPix grid is fixed to the axes; the quadrature error moves
}
#[test]
fn rotating_the_cluster_rotates_the_forces_to_quadrature_accuracy() {
    // rotation about z by 37 degrees then about x by 21 degrees
    let (c1, s1, c2, s2) = (
        (37f64.to_radians()).cos(),
        (37f64.to_radians()).sin(),
        (21f64.to_radians()).cos(),
        (21f64.to_radians()).sin(),
    );
    let rot = |p: &V3| -> V3 {
        let q = [c1 * p[0] - s1 * p[1], s1 * p[0] + c1 * p[1], p[2]];
        [q[0], c2 * q[1] - s2 * q[2], s2 * q[1] + c2 * q[2]]
    };
    for (name, val, bound) in [
        ("standard", None, 1e-3),
        ("validation", Some(Validation::default_validation()), 1e-5),
    ] {
        let w1 = world(&CLUSTER, 3.0, val);
        let rp: Vec<V3> = CLUSTER.iter().map(rot).collect();
        let w2 = world(&rp, 3.0, val);
        let f1r: Vec<V3> = w1.force.f_total.iter().map(rot).collect();
        let d = max_rel(&f1r, &w2.force.f_total);
        eprintln!("[invariance] rotation {name}: {d:e}");
        assert!(
            d < bound,
            "{name}: rotation covariance error {d:e} exceeds {bound:e}"
        );
    }
}
#[test]
fn uniform_scaling_obeys_the_dimensional_law() {
    // pair-additive thin limit: F ~ 1/r^2. Scaling all separations by s at fixed radii (tau_c ~ 0) divides F by s^2.
    let two = |d: f64| world(&[[0.0; 3], [d, 0.0, 0.0]], 1e-6, None).force.f_total[0];
    let (f10, f20) = (norm(&two(10.0)), norm(&two(20.0)));
    assert!(
        (f10 / f20 - 4.0).abs() < 1e-4,
        "1/r^2 law: ratio {}",
        f10 / f20
    );
}
#[test]
fn force_is_finite_and_reciprocal_for_a_dense_random_cluster() {
    // deterministic pseudo-random positions, min spacing > 2.1 a so spheres do not overlap
    let mut seed = 0x9E3779B97F4A7C15u64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut pos: Vec<V3> = Vec::new();
    while pos.len() < 12 {
        let c = [rnd() * 14.0, rnd() * 14.0, rnd() * 14.0];
        if pos.iter().all(|p| norm(&sub(p, &c)) > 2.1) {
            pos.push(c);
        }
    }
    let w = world(&pos, 2.0, Some(Validation::default_validation()));
    for f in &w.force.f_total {
        assert!(f.iter().all(|x| x.is_finite()));
    }
    // net force vanishes only for reciprocal (pair) forces; the non-additive part breaks it by the documented R_P
    let net = norm(&w.force.f_total.iter().fold([0.0; 3], |a, f| add(&a, f)));
    let sum_abs: f64 = w.force.f_total.iter().map(norm).sum();
    eprintln!(
        "[invariance] N=12 dense: |sum F| / sum|F| = {:e}",
        net / sum_abs
    );
    assert!(
        net / sum_abs < 5e-2,
        "momentum residual {:e}",
        net / sum_abs
    );
}
