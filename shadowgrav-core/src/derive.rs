//! Eqs. 1-2. Recompute whenever m, rho, delta or kappa0 change.
use crate::mathx;
use crate::sum::Neum;
use crate::types::*;
use std::f64::consts::PI;

/// Series switch. The Spec says tau<0.05; the series is the exact Taylor expansion, so any value
/// <= 1 is mathematically identical (and avoids ~1e-13 cancellation in the closed form).
pub const SERIES_TAU: f64 = 1.0;

/// A(tau) = 1 - 2/tau^2 [1-(1+tau)e^-tau] = -2 sum_{n>=3} (-1)^n (n-1) tau^(n-2)/n!
pub fn a_fun(t: f64) -> (f64, bool) {
    if t < SERIES_TAU {
        let mut s = Neum::default();
        let mut u = t / 6.0; // tau^(n-2)/n! at n=3
        for n in 3..=40usize {
            let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
            s.add(-2.0 * sign * (n as f64 - 1.0) * u);
            u *= t / (n as f64 + 1.0);
        }
        (s.value(), true)
    } else {
        (1.0 - 2.0 / (t * t) * (1.0 - (1.0 + t) * mathx::exp(-t)), false)
    }
}

pub fn derive(p: &Particles, g: &Globals) -> Result<Derived, CoreError> {
    p.validate()?;
    g.validate()?;
    let n = p.x.len();
    let z = || vec![0.0; n];
    let mut d = Derived { a: z(), kappa: z(), beta: z(), tau_c: z(), a_fun: z(), a_eff: z(),
                          s: z(), m_g: z(), series_used: vec![false; n] };
    for i in 0..n {
        if p.kind[i] == Kind::Probe { d.a_eff[i] = g.kappa0; d.m_g[i] = 1.0; d.s[i] = 1.0; continue; }
        if !(p.m[i] > 0.0) || !(p.rho[i] > 0.0) || !p.m[i].is_finite() || !p.rho[i].is_finite() {
            return Err(CoreError::InvalidParticle(i));
        }
        let a = mathx::cbrt(3.0 * p.m[i] / (4.0 * PI * p.rho[i]));      // eq. 1
        let kappa = g.kappa0 * (1.0 + p.delta[i]);
        let beta = kappa * p.rho[i];
        let tau = 2.0 * beta * a;
        let (af, ser) = a_fun(tau);                                     // eq. 2
        d.a[i] = a; d.kappa[i] = kappa; d.beta[i] = beta; d.tau_c[i] = tau;
        d.a_fun[i] = af; d.series_used[i] = ser;
        d.a_eff[i] = PI * a * a * af;
        d.s[i] = if tau > 0.0 { 3.0 * af / (2.0 * tau) } else { 1.0 };
        d.m_g[i] = (1.0 + p.delta[i]) * p.m[i] * d.s[i];
        if ![a, kappa, beta, tau, af, d.a_eff[i], d.s[i], d.m_g[i]].iter().all(|v| v.is_finite())
            || !(a > 0.0) || !(d.a_eff[i] > 0.0) || !(d.s[i] > 0.0) || !(d.m_g[i] > 0.0) {
            return Err(CoreError::InvalidParticle(i));
        }
    }
    d.validate_shape(n)?;
    Ok(d)
}
