//! Sky map of the occlusion deficit D_k(n) seen by one body (read-only diagnostic).
//!
//! Pure function of `World`: no force evaluation, no mutation, nothing feeds back into the integrator. It uses the
//! SAME per-ray formulas as the force core (eq. 5 / eq. 16 chord; alpha_bg = 0: the cancellation-free recursion of
//! `overlap::corr_static`; alpha_bg > 0: the eq. 17 sweep), evaluated on a pixel grid instead of HEALPix leaves.
//!
//!   D_k(n)         total deficit along direction n (all other bodies as occluders, mutual shadowing included)
//!   D^NA_k(n)      D_k(n) - sum_j D_kj(n)   (<= 0; non-zero only where discs overlap: the non-additive correction)
//!
//! Directions point FROM the target TOWARD the sky (the force on k is the n-weighted integral of the deficit).
//! Two projections: all-sky Mollweide (equal area, longitude = atan2(y,x) increasing to the right, latitude = asin z)
//! and a zoom (azimuthal equidistant) centred on one occluder, needed because physical discs are small
//! (a/d ~ 0.01 is ~0.6 degrees, below an all-sky pixel).
//! Geometry shown is the STATIC one: V2 aberration (shifted discs) is not drawn.
use crate::deficit::{d_single, sweep};
use crate::geom::*;
use crate::mathx;
#[inline] fn sc(x: f64) -> (f64, f64) { mathx::sin_cos(x) }
use crate::state::World;
use crate::types::Kind;
use std::f64::consts::PI;

/// Cost control: pixels x occluders per map. Above ~4x this the map is refused (never silently coarsened further).
pub const SKY_BUDGET: f64 = 1.5e6;
pub const SKY_MIN_N: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkyFocus { All, Occluder(usize) }

#[derive(Clone, Copy, Debug)]
pub enum SkyProjection {
    Mollweide,
    /// Azimuthal equidistant about unit `c`; `e1` = screen right, `e2` = screen up (e1 = up x c, e2 = c x e1).
    Zoom { c: V3, e1: V3, e2: V3, half_fov: f64 },
}

#[derive(Clone, Debug)]
pub struct SkyDisc { pub j: usize, pub centre: V3, pub theta: f64, pub dist: f64, pub interior: bool }

#[derive(Clone, Debug)]
pub struct SkyMap {
    pub k: usize,
    pub nlon: usize, pub nlat: usize,
    pub proj: SkyProjection,
    /// Occluder the zoom is centred on (None for all-sky).
    pub focus: Option<usize>,
    /// Row-major, row 0 = top. NaN outside the projection's footprint.
    pub d_total: Vec<f32>, pub d_na: Vec<f32>,
    pub discs: Vec<SkyDisc>,
    pub n_occluders: usize,
    pub d_max: f64, pub d_max_dir: V3,
    pub na_min: f64,
    /// All-sky only (equal-area mean over the sphere); NaN in zoom.
    pub d_bar: f64, pub covered: f64, pub na_mean: f64,
}

#[inline] fn finite_dir(n: &V3) -> bool { n.iter().all(|x| x.is_finite()) }

impl SkyMap {
    /// width / height of the footprint (2 for Mollweide, 1 for zoom).
    pub fn aspect(&self) -> f32 { self.nlon as f32 / self.nlat as f32 }

    /// Direction -> normalised map coordinates (x right, y up, both in [-1, 1]); None if outside the footprint.
    pub fn project(&self, n: &V3) -> Option<(f64, f64)> {
        if !finite_dir(n) { return None; }
        match self.proj {
            SkyProjection::Mollweide => {
                let lon = mathx::atan2(n[1], n[0]);
                let sl = n[2].clamp(-1.0, 1.0);
                let lat = sl.asin();
                let mut th = lat;
                for _ in 0..30 {
                    let f = 2.0 * th + sc(2.0 * th).0 - PI * sl;
                    let fp = 2.0 + 2.0 * sc(2.0 * th).1;
                    if fp.abs() < 1e-12 { break; }
                    let d = f / fp;
                    th -= d;
                    if d.abs() < 1e-13 { break; }
                }
                Some((lon / PI * sc(th).1, sc(th).0))
            }
            SkyProjection::Zoom { c, e1, e2, half_fov } => {
                let r = psi(n, &c);
                if r > half_fov { return None; }
                let cr = dot(n, &c);
                let p = [n[0] - cr * c[0], n[1] - cr * c[1], n[2] - cr * c[2]];
                let (px, py) = (dot(&p, &e1), dot(&p, &e2));
                if px * px + py * py < 1e-300 { return Some((0.0, 0.0)); }
                let phi = mathx::atan2(py, px);
                let (sp, cp) = sc(phi); Some((r / half_fov * cp, r / half_fov * sp))
            }
        }
    }

    /// Normalised map coordinates -> unit direction; None outside the footprint.
    pub fn unproject(&self, x: f64, y: f64) -> Option<V3> {
        match self.proj {
            SkyProjection::Mollweide => {
                if y.abs() > 1.0 { return None; }
                let th = y.asin();
                let ct = (1.0 - y * y).sqrt();
                if x.abs() > ct { return None; }
                let lon = if ct > 1e-15 { PI * x / ct } else { 0.0 };
                let z = ((2.0 * th + sc(2.0 * th).0) / PI).clamp(-1.0, 1.0);
                let cl = (1.0 - z * z).sqrt();
                let (slon, clon) = sc(lon); Some([cl * clon, cl * slon, z])
            }
            SkyProjection::Zoom { c, e1, e2, half_fov } => {
                let rho = (x * x + y * y).sqrt();
                if rho > 1.0 { return None; }
                if rho < 1e-15 { return Some(c); }
                let r = rho * half_fov;
                let (cp, sp) = (x / rho, y / rho);
                let (sr, cr) = sc(r);
                Some([
                    cr * c[0] + sr * (cp * e1[0] + sp * e2[0]),
                    cr * c[1] + sr * (cp * e1[1] + sp * e2[1]),
                    cr * c[2] + sr * (cp * e1[2] + sp * e2[2]),
                ])
            }
        }
    }

    /// Row-major index of the pixel containing normalised coordinates (x, y), if inside the grid.
    pub fn pixel_at(&self, x: f64, y: f64) -> Option<usize> {
        if x.abs() > 1.0 || y.abs() > 1.0 { return None; }
        let col = (((x + 1.0) * 0.5 * self.nlon as f64).floor() as usize).min(self.nlon - 1);
        let row = (((1.0 - y) * 0.5 * self.nlat as f64).floor() as usize).min(self.nlat - 1);
        Some(row * self.nlon + col)
    }
}

/// Zoom frame about unit `c`: e1 = up x c (longitude increases to the right, as in the all-sky map), e2 = c x e1.
fn zoom_frame(c: &V3) -> (V3, V3) {
    let up = if c[2].abs() > 0.999 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let e1 = normalize(&cross(&up, c));
    let e2 = cross(c, &e1);
    (e1, e2)
}

/// Sky map for body `k`. `fov_factor` (>= 1) is only used for `SkyFocus::Occluder(j)`: half field of view =
/// fov_factor x (angular radius of disc j). Err(reason) is meant to be shown to the person, not hidden.
pub fn sky_deficit_map(w: &World, k: usize, focus: SkyFocus, fov_factor: f64) -> Result<SkyMap, String> {
    if k >= w.n() || w.p.kind[k] != Kind::Body { return Err("selected object is not a body".to_string()); }
    // occluders = all other bodies (same set as the force core)
    let mut occ: Vec<(V3, f64, f64)> = Vec::new();   // (c = x_j - x_k, a_j, beta_j)
    let mut discs: Vec<SkyDisc> = Vec::new();
    for j in 0..w.n() {
        if j == k || w.p.kind[j] != Kind::Body { continue; }
        let c = sub(&w.p.x[j], &w.p.x[k]);
        let d = norm(&c);
        let a = w.dv.a[j];
        let (chat, theta) = if d < 1e-12 * a { ([0.0, 0.0, 1.0], PI) }
            else { (scale(&c, 1.0 / d), if d > a { mathx::atan2(a, ((d - a) * (d + a)).sqrt()) } else { PI }) };
        occ.push((c, a, w.dv.beta[j]));
        discs.push(SkyDisc { j, centre: chat, theta, dist: d, interior: d <= a });
    }
    let m = occ.len();
    let alpha = w.g.alpha_bg;

    let proj = match focus {
        SkyFocus::All => SkyProjection::Mollweide,
        SkyFocus::Occluder(j) => {
            let d = discs.iter().find(|d| d.j == j).ok_or_else(|| format!("occluder {j} is not another body"))?;
            let (e1, e2) = zoom_frame(&d.centre);
            let hf = (fov_factor.max(1.0) * d.theta).clamp(1e-6, PI);
            SkyProjection::Zoom { c: d.centre, e1, e2, half_fov: hf }
        }
    };

    // grid size from the cost budget (pixels x occluders)
    let mm = m.max(1) as f64;
    let (nlon, nlat) = match proj {
        SkyProjection::Mollweide => {
            let nlat = ((SKY_BUDGET / (2.0 * mm)).sqrt().floor() as usize).min(120);
            (2 * nlat, nlat)
        }
        SkyProjection::Zoom { .. } => {
            let n = ((SKY_BUDGET / mm).sqrt().floor() as usize).min(200);
            (n, n)
        }
    };
    if nlat < SKY_MIN_N {
        return Err(format!("{m} occluders: live sky map disabled (cost budget); select fewer bodies or lower N"));
    }

    let mut map = SkyMap {
        k, nlon, nlat, proj, focus: match focus { SkyFocus::All => None, SkyFocus::Occluder(j) => Some(j) },
        d_total: vec![f32::NAN; nlon * nlat], d_na: vec![f32::NAN; nlon * nlat], discs,
        n_occluders: m, d_max: 0.0, d_max_dir: [1.0, 0.0, 0.0], na_min: 0.0,
        d_bar: f64::NAN, covered: f64::NAN, na_mean: f64::NAN,
    };

    let mut iv: Vec<(f64, f64, f64)> = Vec::new();
    let (mut cnt, mut sum_d, mut sum_na, mut cov) = (0u64, 0.0f64, 0.0f64, 0u64);
    for row in 0..nlat {
        let y = 1.0 - 2.0 * (row as f64 + 0.5) / nlat as f64;
        for col in 0..nlon {
            let x = 2.0 * (col as f64 + 0.5) / nlon as f64 - 1.0;
            let Some(n) = map.unproject(x, y) else { continue };
            let (dt, y_na) = if alpha == 0.0 {
                // cancellation-free: Y <- Y - X D_j ; X <- X + D_j (1 - X)   (as overlap::corr_static)
                let (mut xx, mut yy) = (0.0f64, 0.0f64);
                for &(c, a, beta) in &occ {
                    if let Some((_, _, l)) = chord(&c, a, &n) {
                        let dj = -mathx::expm1(-beta * l);
                        yy -= xx * dj; xx += dj * (1.0 - xx);
                    }
                }
                (xx, yy)
            } else {
                iv.clear();
                let mut s = 0.0f64;
                for &(c, a, beta) in &occ {
                    if let Some((s1, s2, l)) = chord(&c, a, &n) {
                        s += d_single(alpha, beta, s1, l);
                        iv.push((s1.max(0.0), s2, beta));
                    }
                }
                let dt = sweep(&iv, alpha);
                (dt, dt - s)
            };
            let idx = row * nlon + col;
            map.d_total[idx] = dt as f32;
            map.d_na[idx] = y_na as f32;
            cnt += 1; sum_d += dt; sum_na += y_na;
            if dt > 0.0 { cov += 1; }
            if dt > map.d_max { map.d_max = dt; map.d_max_dir = n; }
            if y_na < map.na_min { map.na_min = y_na; }
        }
    }
    if matches!(map.proj, SkyProjection::Mollweide) && cnt > 0 {
        // Mollweide is equal-area, so the pixel mean is the sphere mean
        map.d_bar = sum_d / cnt as f64;
        map.na_mean = sum_na / cnt as f64;
        map.covered = cov as f64 / cnt as f64;
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::*;
    use crate::state::{SimConfig, World};

    fn orbit_world() -> World { two_body_orbit(1e-6, 1000, crate::Mode::V0, 1e3).unwrap().world }

    #[test] fn projection_roundtrip() {
        let w = orbit_world();
        for focus in [SkyFocus::All, SkyFocus::Occluder(1)] {
            let m = sky_deficit_map(&w, 0, focus, 4.0).unwrap();
            for &(x, y) in &[(0.1, 0.2), (-0.4, 0.3), (0.0, 0.0), (0.3, -0.5)] {
                let n = m.unproject(x, y).unwrap();
                assert!((norm(&n) - 1.0).abs() < 1e-12);
                let (px, py) = m.project(&n).unwrap();
                assert!((px - x).abs() < 1e-9 && (py - y).abs() < 1e-9, "{focus:?}: ({x},{y}) -> ({px},{py})");
            }
        }
    }

    #[test] fn two_body_has_no_nonadditive_part_and_deficit_peaks_toward_the_other_body() {
        let w = orbit_world();
        let m = sky_deficit_map(&w, 0, SkyFocus::Occluder(1), 4.0).unwrap();
        assert!(m.d_na.iter().filter(|v| v.is_finite()).all(|&v| v == 0.0), "single disc: D == sum D_j exactly");
        let c = &m.discs[0].centre;
        assert!(dot(&m.d_max_dir, c) > 0.9999, "peak deficit must sit inside the occluder's disc");
        assert!(m.d_max > 0.0);
    }

    /// The support of D^NA is exactly the set of directions covered by >= 2 discs: pixel by pixel, no non-additive part
    /// where at most one disc can cover (with a 1e-6 relative rim margin for the chord-vs-cone edge), and a strictly
    /// negative one wherever two or more discs definitely cover. Checked on the zoom and all-sky maps of (c2) and (c3).
    #[test] fn nonadditive_support_is_exactly_the_overlap() {
        let a = sphere_radius(1.0, RHO0);
        for third in [[12.0, 2.0, 0.0], [12.0, 1.4, 1.4]] {
            let mut g = crate::Globals::defaults();
            g.kappa0 = kappa0_for_tau(5.0, 1.0, RHO0);
            let xs: Vec<V3> = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], third].iter().map(|p| scale(p, a)).collect();
            let w = World::new(body_particles(xs, vec![[0.0; 3]; 3]), g, default_options(crate::Mode::V0), SimConfig::fixed(1e-4)).unwrap();
            for focus in [SkyFocus::Occluder(1), SkyFocus::All] {
                let m = sky_deficit_map(&w, 0, focus, 3.0).unwrap();
                let (mut n_overlap, mut n_single) = (0usize, 0usize);
                for row in 0..m.nlat {
                    let y = 1.0 - 2.0 * (row as f64 + 0.5) / m.nlat as f64;
                    for col in 0..m.nlon {
                        let x = 2.0 * (col as f64 + 0.5) / m.nlon as f64 - 1.0;
                        let Some(n) = m.unproject(x, y) else { continue };
                        let v = m.d_na[row * m.nlon + col] as f64;
                        let ang = |d: &SkyDisc| dot(&n, &d.centre).clamp(-1.0, 1.0).acos();
                        let maybe = m.discs.iter().filter(|d| ang(d) < d.theta * (1.0 + 1e-6)).count();
                        let sure = m.discs.iter().filter(|d| ang(d) < d.theta * (1.0 - 1e-6)).count();
                        if maybe <= 1 { assert_eq!(v, 0.0, "{third:?} {focus:?}: D^NA != 0 outside the overlap at ({x:.3},{y:.3})"); n_single += 1; }
                        if sure >= 2 { assert!(v < 0.0, "{third:?} {focus:?}: D^NA == 0 inside the overlap at ({x:.3},{y:.3})"); n_overlap += 1; }
                    }
                }
                if matches!(focus, SkyFocus::Occluder(_)) { assert!(n_overlap > 20 && n_single > 20, "overlap {n_overlap}, single {n_single}"); }
            }
        }
    }

    #[test] fn nonadditive_part_is_never_positive() {
        // 3 bodies, partial overlap (same geometry family as tests/oracle_sky.rs)
        let a = sphere_radius(1.0, RHO0);
        let mut g = crate::Globals::defaults();
        g.kappa0 = kappa0_for_tau(5.0, 1.0, RHO0);
        let xs: Vec<V3> = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]].iter().map(|p| scale(p, a)).collect();
        let w = World::new(body_particles(xs, vec![[0.0; 3]; 3]), g, default_options(crate::Mode::V0), SimConfig::fixed(1e-4)).unwrap();
        for focus in [SkyFocus::All, SkyFocus::Occluder(1)] {
            let m = sky_deficit_map(&w, 0, focus, 3.0).unwrap();
            assert!(m.d_na.iter().filter(|v| v.is_finite()).all(|&v| v <= 1e-12), "Y = D - sum D_j <= 0");
        }
        // zoomed on the nearer disc the overlap with the farther one is resolved
        let m = sky_deficit_map(&w, 0, SkyFocus::Occluder(1), 3.0).unwrap();
        assert!(m.na_min < -1e-3, "overlap region must be visible in the zoom (na_min = {})", m.na_min);
    }
}
