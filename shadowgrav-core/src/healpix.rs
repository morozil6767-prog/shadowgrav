//! HEALPix NESTED pixel centres (Spec 4.3 item 3). Hand-transcribed from the chealpix
//! pix2ang_nest algorithm. Cross-checked in Python against the reference values (see VERIFY.md);
//! tests/healpix_ref.rs repeats the checks in Rust.
use std::f64::consts::PI;
const JRLL: [i64; 12] = [2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4]; // ring offset of face
const JPLL: [i64; 12] = [1, 3, 5, 7, 0, 2, 4, 6, 1, 3, 5, 7]; // phi offset of face

/// Extract even-indexed bits (de-interleave Morton code).
fn compress(mut v: u64) -> u64 {
    v &= 0x5555_5555_5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333_3333_3333;
    v = (v | (v >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
    v = (v | (v >> 4)) & 0x00ff_00ff_00ff_00ff;
    v = (v | (v >> 8)) & 0x0000_ffff_0000_ffff;
    v = (v | (v >> 16)) & 0x0000_0000_ffff_ffff;
    v
}

/// Unit vector of nested pixel `pix` at `nside` (power of two, pix < 12 nside^2).
pub fn pix2vec_nest(nside: u64, pix: u64) -> [f64; 3] {
    debug_assert!(nside.is_power_of_two() && pix < 12 * nside * nside);
    let order = nside.trailing_zeros();
    let face = (pix / (nside * nside)) as usize;
    let ipf = pix % (nside * nside);
    let ix = compress(ipf) as i64;
    let iy = compress(ipf >> 1) as i64;
    let ns = nside as i64;
    let nl4 = 4 * ns;
    let jr: i64 = (JRLL[face] << order) - ix - iy - 1;
    let fact2 = 4.0 / (12.0 * (ns as f64) * (ns as f64));
    let fact1 = ((2 * ns) as f64) * fact2;
    let nr: i64;
    let z: f64;
    let sth: f64;
    let kshift: i64;
    if jr < ns {
        nr = jr;
        let x = nr as f64;
        z = 1.0 - x * x * fact2;
        sth = x * (fact2 * (2.0 - x * x * fact2)).sqrt();
        kshift = 0;
    } else if jr > 3 * ns {
        nr = nl4 - jr;
        let x = nr as f64;
        z = x * x * fact2 - 1.0;
        sth = x * (fact2 * (2.0 - x * x * fact2)).sqrt();
        kshift = 0;
    } else {
        nr = ns;
        z = ((2 * ns - jr) as f64) * fact1;
        sth = ((1.0 - z) * (1.0 + z)).sqrt();
        kshift = (jr - ns) & 1;
    }
    let mut jp = (JPLL[face] * nr + ix - iy + 1 + kshift) / 2;
    if jp > nl4 { jp -= nl4; }
    if jp < 1 { jp += nl4; }
    let phi = (jp as f64 - (kshift + 1) as f64 * 0.5) * ((PI / 2.0) / nr as f64);
    let (sp, cp) = crate::mathx::sin_cos(phi);   // platform-independent under `vendored-libm` (Addendum 3 item 10)
    [sth * cp, sth * sp, z]
}
/// A_l = pi/(3 Nside^2); rho_l = 1.5 sqrt(A_l)   (Spec 4.3 item 3)
pub fn cell_area(nside: u64) -> f64 { PI / (3.0 * (nside as f64) * (nside as f64)) }
pub fn cell_radius(area: f64) -> f64 { 1.5 * area.sqrt() }
