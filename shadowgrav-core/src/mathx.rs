//! Every transcendental used by the core goes through here (Architecture R11).
//! Default: std (platform libm). Feature `vendored-libm`: pure-Rust libm, reproducible across platforms.
//! sqrt is IEEE-exact everywhere and is used directly.
#[cfg(feature = "vendored-libm")]
mod imp {
    pub fn expm1(x: f64) -> f64 {
        libm::expm1(x)
    }
    pub fn exp(x: f64) -> f64 {
        libm::exp(x)
    }
    pub fn atan2(y: f64, x: f64) -> f64 {
        libm::atan2(y, x)
    }
    pub fn sin_cos(x: f64) -> (f64, f64) {
        (libm::sin(x), libm::cos(x))
    }
    pub fn cbrt(x: f64) -> f64 {
        libm::cbrt(x)
    }
    pub fn sinh(x: f64) -> f64 {
        libm::sinh(x)
    }
    pub fn asinh(x: f64) -> f64 {
        libm::asinh(x)
    }
    pub fn cos(x: f64) -> f64 {
        libm::cos(x)
    }
    pub fn ln(x: f64) -> f64 {
        libm::log(x)
    }
}
#[cfg(not(feature = "vendored-libm"))]
mod imp {
    pub fn expm1(x: f64) -> f64 {
        x.exp_m1()
    }
    pub fn exp(x: f64) -> f64 {
        x.exp()
    }
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
    pub fn sin_cos(x: f64) -> (f64, f64) {
        x.sin_cos()
    }
    pub fn cbrt(x: f64) -> f64 {
        x.cbrt()
    }
    pub fn sinh(x: f64) -> f64 {
        x.sinh()
    }
    pub fn asinh(x: f64) -> f64 {
        x.asinh()
    }
    pub fn cos(x: f64) -> f64 {
        x.cos()
    }
    pub fn ln(x: f64) -> f64 {
        x.ln()
    }
}
pub use imp::*;

/// `!(left > right)`, including when either side is NaN.
///
/// `left <= right` is not a substitute: a NaN comparison is false, so the negation is true,
/// while `<=` is false for NaN. `partial_cmp` keeps that result.
#[inline]
pub fn not_gt(left: f64, right: f64) -> bool {
    left.partial_cmp(&right) != Some(std::cmp::Ordering::Greater)
}

/// `!(left < right)`, including when either side is NaN.
#[inline]
pub fn not_lt(left: f64, right: f64) -> bool {
    left.partial_cmp(&right) != Some(std::cmp::Ordering::Less)
}

/// `!(left <= right)`, including when either side is NaN.
#[inline]
pub fn not_le(left: f64, right: f64) -> bool {
    !matches!(
        left.partial_cmp(&right),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

#[cfg(test)]
mod cmp_equiv {
    use super::{not_gt, not_le, not_lt};

    /// Expected truth table for the negated comparisons, including NaN.
    /// A NaN comparison is unordered, so each negated form is true when either side is NaN.
    #[test]
    fn helpers_match_the_nan_aware_truth_table() {
        let finite = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f64::MIN,
            f64::MAX,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for &a in &finite {
            for &b in &finite {
                let ord = a.partial_cmp(&b).unwrap();
                assert_eq!(
                    not_gt(a, b),
                    ord != std::cmp::Ordering::Greater,
                    "{a} gt {b}"
                );
                assert_eq!(not_lt(a, b), ord != std::cmp::Ordering::Less, "{a} lt {b}");
                assert_eq!(
                    not_le(a, b),
                    ord == std::cmp::Ordering::Greater,
                    "{a} le {b}"
                );
            }
        }
        for &x in &[0.0, 1.0, -1.0, f64::INFINITY] {
            assert!(not_gt(f64::NAN, x) && not_gt(x, f64::NAN));
            assert!(not_lt(f64::NAN, x) && not_lt(x, f64::NAN));
            assert!(not_le(f64::NAN, x) && not_le(x, f64::NAN));
        }
        assert!(
            not_gt(f64::NAN, f64::NAN) && not_lt(f64::NAN, f64::NAN) && not_le(f64::NAN, f64::NAN)
        );
    }
}
