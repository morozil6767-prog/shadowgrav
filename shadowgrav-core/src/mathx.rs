//! Every transcendental used by the core goes through here (Architecture R11).
//! Default: std (platform libm). Feature `vendored-libm`: pure-Rust libm, reproducible across platforms.
//! sqrt is IEEE-exact everywhere and is used directly.
#[cfg(feature = "vendored-libm")]
mod imp {
    pub fn expm1(x: f64) -> f64 { libm::expm1(x) }
    pub fn exp(x: f64) -> f64 { libm::exp(x) }
    pub fn atan2(y: f64, x: f64) -> f64 { libm::atan2(y, x) }
    pub fn sin_cos(x: f64) -> (f64, f64) { (libm::sin(x), libm::cos(x)) }
    pub fn cbrt(x: f64) -> f64 { libm::cbrt(x) }
    pub fn sinh(x: f64) -> f64 { libm::sinh(x) }
    pub fn asinh(x: f64) -> f64 { libm::asinh(x) }
    pub fn cos(x: f64) -> f64 { libm::cos(x) }
    pub fn ln(x: f64) -> f64 { libm::log(x) }
}
#[cfg(not(feature = "vendored-libm"))]
mod imp {
    pub fn expm1(x: f64) -> f64 { x.exp_m1() }
    pub fn exp(x: f64) -> f64 { x.exp() }
    pub fn atan2(y: f64, x: f64) -> f64 { y.atan2(x) }
    pub fn sin_cos(x: f64) -> (f64, f64) { x.sin_cos() }
    pub fn cbrt(x: f64) -> f64 { x.cbrt() }
    pub fn sinh(x: f64) -> f64 { x.sinh() }
    pub fn asinh(x: f64) -> f64 { x.asinh() }
    pub fn cos(x: f64) -> f64 { x.cos() }
    pub fn ln(x: f64) -> f64 { x.ln() }
}
pub use imp::*;
