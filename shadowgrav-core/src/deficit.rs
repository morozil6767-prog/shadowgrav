//! Single-occluder deficit (eq. 5) and ordered ray-transfer sweep (eq. 17).
use crate::mathx;

/// Eq. 5. s1 = near-entry distance, l = chord length.
#[inline]
pub fn d_single(alpha: f64, beta: f64, s1: f64, l: f64) -> f64 {
    let lam = alpha + beta;
    if crate::mathx::not_gt(lam, 0.0) {
        return 0.0;
    }
    let pre = if alpha > 0.0 {
        mathx::exp(-alpha * s1.max(0.0))
    } else {
        1.0
    };
    pre * (beta / lam) * (-mathx::expm1(-lam * l))
}

/// Eq. 17 (alpha>0). iv = (s1 clipped to >=0, s2>0, beta). Far-to-near sweep, then gap factor.
pub fn sweep(iv: &[(f64, f64, f64)], alpha: f64) -> f64 {
    if iv.is_empty() {
        return 0.0;
    }
    let mut bp: Vec<f64> = Vec::with_capacity(2 * iv.len());
    for &(s1, s2, _) in iv {
        bp.push(s1);
        bp.push(s2);
    }
    bp.sort_by(|a, b| b.total_cmp(a)); // descending; NaN-safe
    bp.dedup();
    let mut dd = 0.0;
    for w in bp.windows(2) {
        let (hi, lo) = (w[0], w[1]);
        let l = hi - lo;
        let mut bs = crate::sum::Neum::default(); // beta_seg recomputed per segment
        for &(s1, s2, b) in iv {
            if s1 <= lo && s2 >= hi {
                bs.add(b);
            }
        }
        let lam = alpha + bs.value(); // > 0 because alpha > 0
        dd = (bs.value() / lam) * (-mathx::expm1(-lam * l)) + dd * mathx::exp(-lam * l);
    }
    dd * mathx::exp(-alpha * *bp.last().unwrap()) // gap factor, s_near = smallest endpoint
}
