//! Stage 1. HEALPix reference tests. Mirrors the Python cross-check recorded in VERIFY.md.
use shadowgrav_core::geom::*;
use shadowgrav_core::healpix::*;
use std::f64::consts::PI;

fn zphi(v: &V3) -> (f64, f64) { (v[2], v[1].atan2(v[0]).rem_euclid(2.0 * PI)) }

#[test] fn base_centres_nside1() {
    for p in 0..12u64 {
        let (z, phi) = zphi(&pix2vec_nest(1, p));
        let k = (p % 4) as f64;
        let (ez, ephi) = match p / 4 {
            0 => (2.0 / 3.0, PI / 4.0 + PI / 2.0 * k),
            1 => (0.0, PI / 2.0 * k),
            _ => (-2.0 / 3.0, PI / 4.0 + PI / 2.0 * k),
        };
        assert!((z - ez).abs() < 1e-14, "pix {p} z={z}");
        let dphi = ((phi - ephi + PI).rem_euclid(2.0 * PI) - PI).abs();
        assert!(dphi < 1e-13, "pix {p} phi={phi}");
    }
}
#[test] fn nside2_children_of_face0() {
    let mut zs: Vec<f64> = (0..4).map(|p| pix2vec_nest(2, p)[2]).collect();
    zs.sort_by(|a, b| a.total_cmp(b));
    for (z, e) in zs.iter().zip([1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 11.0 / 12.0]) { assert!((z - e).abs() < 1e-14); }
}
#[test] fn unit_norm_all_levels() {
    for lvl in 0..4u32 { let ns = 8u64 << lvl;
        for pix in 0..12 * ns * ns { assert!((norm(&pix2vec_nest(ns, pix)) - 1.0).abs() < 1e-13); } }
}
fn moments(ns: u64) -> (V3, f64, f64, f64) {
    let a = cell_area(ns);
    let (mut s, mut zz, mut xx, mut yy) = ([0.0; 3], 0.0, 0.0, 0.0);
    for pix in 0..12 * ns * ns {
        let n = pix2vec_nest(ns, pix);
        for i in 0..3 { s[i] += a * n[i]; }
        zz += a * n[2] * n[2]; xx += a * n[0] * n[0]; yy += a * n[1] * n[1];
    }
    (s, zz, xx, yy)
}
#[test] fn equal_area_quadrature_and_symmetry() {
    let exact = 4.0 * PI / 3.0;
    let mut errs = vec![];
    for &ns in &[16u64, 64] {
        let (s, zz, xx, yy) = moments(ns);
        assert!(s.iter().all(|c| c.abs() < 1e-11));
        assert!((xx - yy).abs() < 1e-11);
        assert!((xx + yy + zz - 4.0 * PI).abs() < 1e-10);
        errs.push((zz - exact).abs() / exact);
    }
    // Python reference run: 3.6e-4 (ns=16), 2.3e-5 (ns=64): ratio ~16 = (64/16)^2
    assert!(errs[1] < 3e-3 && errs[1] < errs[0] / 8.0, "expect ~ns^-2 convergence: {errs:?}");
}
#[test] fn bounding_radius_covers_descendants() {
    for lvl in 0..2u32 {
        let ns = 8u64 << lvl;
        let rho = cell_radius(cell_area(ns));
        let rho_deep = cell_radius(cell_area(ns << 4));
        let mut mx: f64 = 0.0;
        for pix in 0..12 * ns * ns {
            let c = pix2vec_nest(ns, pix);
            for q in 0..256u64 { mx = mx.max(psi(&c, &pix2vec_nest(ns << 4, pix * 256 + q))); }
        }
        // Python reference run: lvl0 0.134 <= 0.192, lvl1 0.068 <= 0.096 (margin ~30%)
        assert!(mx + rho_deep <= rho, "level {lvl}: max descendant {mx} vs rho {rho}");
    }
}
