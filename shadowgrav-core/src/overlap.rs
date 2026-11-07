//! Section 4.3 adaptive HEALPix correction + Addendum I.4 (separate static / shifted touching sets)
//! + Addendum II.4 (validation-mode silhouette rule).
//! Eq. 4 reading (hardening Q1): D_k and sum_{T_c} D_kj are BOTH evaluated over the candidate set C_k.
use crate::deficit::{d_single, sweep};
use crate::geom::*;
use crate::healpix::*;
use crate::mathx;
use crate::sum::{Neum, Neum3};
use crate::types::*;

/// Occluder as seen from one target.
#[derive(Clone, Debug)]
pub struct Occ {
    pub c: V3, pub chat: V3, pub d: f64, pub a: f64, pub beta: f64,
    pub theta: f64, pub theta_eff: f64,        // theta_eff = min(theta, 1)  (4.3 item 5)
    pub eps: V3, pub e: f64,                   // V2: eps_j
    pub sh_c: V3, pub sh_theta: f64,           // V2: shifted disc (I.4)
}

pub struct Ctx<'a> {
    pub occ: &'a [Occ], pub alpha: f64, pub v2: bool, pub eps_k: V3,
    pub nside0: u64, pub ell_cap: u32, pub f: f64, pub val: Option<Validation>,
    pub budget: u64, pub record: bool, pub k: u32,
}
pub struct Acc {
    pub corr: Neum3, pub ab: Neum3, pub leaves: u64, pub cap_hits: u64,
    pub s0: Vec<Vec<u32>>, pub s1: Vec<Vec<u32>>, pub iv: Vec<(f64, f64, f64)>,
    pub rec: Vec<LeafRecord>,
}
impl Acc {
    pub fn new(ell_cap: u32) -> Acc {
        let n = ell_cap as usize + 2;
        Acc { corr: Neum3::default(), ab: Neum3::default(), leaves: 0, cap_hits: 0,
              s0: vec![Vec::new(); n], s1: vec![Vec::new(); n], iv: Vec::new(), rec: Vec::new() }
    }
}

/// Step 2 of 4.3 (+ I.4): candidate set C_k = nodes with degree >= 1 in the static graph
/// or (V2) the shifted graph. Edge iff angle < theta_j + theta_l. Reference O(m^2).
pub fn candidates_brute(occ: &[Occ], v2: bool) -> Vec<usize> {
    let m = occ.len();
    let mut inc = vec![false; m];
    for p in 0..m {
        for q in (p + 1)..m {
            let (a, b) = (&occ[p], &occ[q]);
            let mut edge = false;
            let ts = a.theta + b.theta;
            if ts >= std::f64::consts::PI { edge = true; }
            else if !(dot(&a.chat, &b.chat) < 0.0 && ts < std::f64::consts::FRAC_PI_2) {
                edge = psi(&a.chat, &b.chat) < ts;
            }
            if !edge && v2 {
                let ts2 = a.sh_theta + b.sh_theta;
                edge = ts2 >= std::f64::consts::PI || psi(&a.sh_c, &b.sh_c) < ts2;
            }
            if edge { inc[p] = true; inc[q] = true; }
        }
    }
    (0..m).filter(|&i| inc[i]).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Broad phase (Spec 4.4 "spatial hash of discs"): EXACT equivalent of `candidates_brute`, O(m log m) per target.
// Edge (p,q) requires psi(c_p,c_q) < theta_p + theta_q. For discs with theta <= BIG_DISC let h = 2 theta_max; then
// |c_p - c_q|_inf <= chord <= psi < theta_p + theta_q <= h, so p and q lie in the same or adjacent cells of a cubic
// grid of side h on the unit vectors. Discs with theta > BIG_DISC (few: close or interior spheres) are tested against
// all others with the SAME predicate. The predicates are shared with the brute path, so only the pruning can differ,
// and the pruning is conservative (tests/perf_equiv.rs checks equality on random sets, including V2 shifted discs).
// ---------------------------------------------------------------------------------------------------------------
const BIG_DISC: f64 = 0.3;
const HASH_MIN: usize = 64;

fn edge_static(a: &Occ, b: &Occ) -> bool {
    let ts = a.theta + b.theta;
    if ts >= std::f64::consts::PI { true }
    else if !(dot(&a.chat, &b.chat) < 0.0 && ts < std::f64::consts::FRAC_PI_2) { psi(&a.chat, &b.chat) < ts }
    else { false }
}
fn edge_shift(a: &Occ, b: &Occ) -> bool {
    let ts2 = a.sh_theta + b.sh_theta;
    ts2 >= std::f64::consts::PI || psi(&a.sh_c, &b.sh_c) < ts2
}

fn mark_grid(occ: &[Occ], centre: impl Fn(&Occ) -> (V3, f64), edge: impl Fn(&Occ, &Occ) -> bool, inc: &mut [bool]) {
    let m = occ.len();
    let geom: Vec<(V3, f64)> = occ.iter().map(|o| centre(o)).collect();
    let (mut big, mut tmax) = (Vec::new(), 0.0f64);
    for i in 0..m { if geom[i].1 > BIG_DISC { big.push(i); } else { tmax = tmax.max(geom[i].1); } }
    for &p in &big { for q in 0..m { if q != p && edge(&occ[p], &occ[q]) { inc[p] = true; inc[q] = true; } } }
    if big.len() == m { return; }
    let h = (2.0 * tmax).max(1e-3);
    let idx = |v: &V3| -> [i64; 3] { [(v[0] / h).floor() as i64, (v[1] / h).floor() as i64, (v[2] / h).floor() as i64] };
    const OFF: i64 = 1 << 20;                                           // |index| <= 1000 because |v| <= 1, h >= 1e-3
    let pack = |c: [i64; 3]| -> u64 { (((c[0] + OFF) as u64) << 42) | (((c[1] + OFF) as u64) << 21) | ((c[2] + OFF) as u64) };
    let mut cells: Vec<(u64, usize)> = (0..m).filter(|&i| geom[i].1 <= BIG_DISC).map(|i| (pack(idx(&geom[i].0)), i)).collect();
    cells.sort_unstable();
    for p in 0..m {
        if geom[p].1 > BIG_DISC { continue; }
        let c = idx(&geom[p].0);
        for dx in -1..=1i64 { for dy in -1..=1i64 { for dz in -1..=1i64 {
            let key = pack([c[0] + dx, c[1] + dy, c[2] + dz]);
            let mut t = cells.partition_point(|e| e.0 < key);
            while t < cells.len() && cells[t].0 == key {
                let q = cells[t].1;
                if q > p && edge(&occ[p], &occ[q]) { inc[p] = true; inc[q] = true; }
                t += 1;
            }
        } } }
    }
}

/// Candidate set C_k. `brute` forces the O(m^2) reference; small m always uses it.
pub fn candidates_with(occ: &[Occ], v2: bool, brute: bool) -> Vec<usize> {
    if brute || occ.len() < HASH_MIN { return candidates_brute(occ, v2); }
    let mut inc = vec![false; occ.len()];
    mark_grid(occ, |o| (o.chat, o.theta), edge_static, &mut inc);
    if v2 { mark_grid(occ, |o| (o.sh_c, o.sh_theta), edge_shift, &mut inc); }
    (0..occ.len()).filter(|&i| inc[i]).collect()
}
pub fn candidates(occ: &[Occ], v2: bool) -> Vec<usize> { candidates_with(occ, v2, false) }

/// Returns (D_total, sum D_j, Y = D - sum D_j). alpha==0: cancellation-free recursion
/// Y <- Y - X D_j ; X <- X + D_j (1-X).  alpha>0: eq. 17 sweep, direct difference.
fn corr_static(cx: &Ctx, iv: &mut Vec<(f64, f64, f64)>, n: &V3, t: &[u32]) -> (f64, f64, f64) {
    let mut sum = Neum::default();
    if cx.alpha == 0.0 {
        let (mut x, mut y) = (0.0, 0.0);
        for &i in t {
            let o = &cx.occ[i as usize];
            if let Some((_, _, l)) = chord(&o.c, o.a, n) {
                let dj = -mathx::expm1(-o.beta * l);                // eq. 5, alpha=0
                y -= x * dj; x += dj * (1.0 - x); sum.add(dj);
            }
        }
        (x, sum.value(), y)
    } else {
        iv.clear();
        for &i in t {
            let o = &cx.occ[i as usize];
            if let Some((s1, s2, l)) = chord(&o.c, o.a, n) {
                sum.add(d_single(cx.alpha, o.beta, s1, l));
                iv.push((s1.max(0.0), s2, o.beta));
            }
        }
        let dt = sweep(iv, cx.alpha);
        (dt, sum.value(), dt - sum.value())
    }
}
/// V2 shifted geometry: sphere j sees direction q_j = (n+eps_j)/|n+eps_j|  (eq. 9). alpha_bg = 0.
fn corr_shifted(cx: &Ctx, n: &V3, t: &[u32]) -> (f64, f64, f64) {
    let (mut x, mut y) = (0.0, 0.0);
    let mut sum = Neum::default();
    for &i in t {
        let o = &cx.occ[i as usize];
        let q = normalize(&add(n, &o.eps));
        if let Some((_, _, l)) = chord(&o.c, o.a, &q) {
            let dj = -mathx::expm1(-o.beta * l);
            y -= x * dj; x += dj * (1.0 - x); sum.add(dj);
        }
    }
    (x, sum.value(), y)
}

pub fn visit(cx: &Ctx, acc: &mut Acc, level: u32, pix: u64, p0: &[u32], p1: &[u32]) -> Result<(), CoreError> {
    let ns = cx.nside0 << level;
    let area = cell_area(ns);
    let rho = cell_radius(area);
    let n = pix2vec_nest(ns, pix);
    let lv = level as usize;
    let mut t0 = std::mem::take(&mut acc.s0[lv]); t0.clear();
    let mut t1 = std::mem::take(&mut acc.s1[lv]); t1.clear();
    // Touching sets T_c = { j : psi_cj - theta_j <= rho }, inherited from parent (4.3 item 4)
    for &i in p0 { let o = &cx.occ[i as usize]; if psi(&n, &o.chat) - o.theta <= rho { t0.push(i); } }
    if cx.v2 { for &i in p1 { let o = &cx.occ[i as usize]; if psi(&n, &o.sh_c) - o.sh_theta <= rho { t1.push(i); } } }
    let big0 = t0.len() >= 2;
    let big1 = cx.v2 && t1.len() >= 2;
    let res = if !big0 && !big1 { Ok(()) }                       // exact zero (4.3 item 5 / I.4)
              else { cell_body(cx, acc, level, pix, &n, area, rho, &t0, &t1, big0, big1) };
    acc.s0[lv] = t0; acc.s1[lv] = t1;
    res
}

#[allow(clippy::too_many_arguments)]
fn cell_body(cx: &Ctx, acc: &mut Acc, level: u32, pix: u64, n: &V3, area: f64, rho: f64,
             t0: &[u32], t1: &[u32], big0: bool, big1: bool) -> Result<(), CoreError> {
    // Leaf test: theta_min over T0 u T^eps with UNSHIFTED theta_eff (I.4)
    let mut tmin = f64::INFINITY;
    for &i in t0.iter().chain(t1.iter()) { tmin = tmin.min(cx.occ[i as usize].theta_eff); }
    let f_int = cx.val.map(|v| v.f_v).unwrap_or(cx.f);
    let mut ok = rho <= f_int * tmin;
    if let Some(v) = cx.val {                                     // II.4 silhouette rule
        let pi = std::f64::consts::PI;
        let mut sil = f64::INFINITY;
        // whole-sky discs (theta >= pi) have no silhouette (hardening N5)
        for &i in t0 { let o = &cx.occ[i as usize];
            if o.theta < pi && (psi(n, &o.chat) - o.theta).abs() <= rho { sil = sil.min(o.theta_eff); } }
        if cx.v2 { for &i in t1 { let o = &cx.occ[i as usize];
            if o.sh_theta < pi && (psi(n, &o.sh_c) - o.sh_theta).abs() <= rho { sil = sil.min(o.theta_eff); } } }
        if sil.is_finite() && !(rho <= v.f_s * sil) { ok = false; }
    }
    if ok || level >= cx.ell_cap {
        if !ok { acc.cap_hits += 1; }                             // cap-hit flag (4.3)
        return leaf(cx, acc, level, pix, n, area, t0, t1, big0, big1);
    }
    for q in 0..4u64 { visit(cx, acc, level + 1, 4 * pix + q, t0, t1)?; }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn leaf(cx: &Ctx, acc: &mut Acc, level: u32, pix: u64, n: &V3, area: f64,
        t0: &[u32], t1: &[u32], big0: bool, big1: bool) -> Result<(), CoreError> {
    acc.leaves += 1;
    if cx.budget > 0 && acc.leaves > cx.budget { return Err(CoreError::LeafBudget { target: cx.k as usize }); }
    let mut rec = LeafRecord { k: cx.k, level: level as u8, pix, n_touch: t0.len() as u16,
                               n_touch_eps: t1.len() as u16, ..Default::default() };
    let mut corr0 = 0.0;
    if big0 {                                                     // eq. 4 leaf: A_l n_c (D - sum_T D_kj)
        let mut iv = std::mem::take(&mut acc.iv);
        let (dt, ds, y) = corr_static(cx, &mut iv, n, t0);
        acc.iv = iv;
        corr0 = y; rec.d_total = dt; rec.d_pair_sum = ds;
        acc.corr.add_scaled(area * corr0, n);
    }
    if cx.v2 {                                                    // I.2: A_l { g_c [..eps..] - n_c [..static..] }
        let mut corr1 = 0.0;
        if big1 {
            let (dt, ds, y) = corr_shifted(cx, n, t1);
            corr1 = y; rec.d_eps = dt; rec.d_pair_sum_eps = ds;
        }
        let gc = gvec(n, &cx.eps_k);
        acc.ab.add_scaled(area * corr1, &gc);
        acc.ab.add_scaled(-area * corr0, n);
    }
    if cx.record { acc.rec.push(rec); }
    Ok(())
}
