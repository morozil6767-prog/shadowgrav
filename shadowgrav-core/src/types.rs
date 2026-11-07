use crate::geom::V3;
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Mode { V0, V1, V2 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Kind { Body, Probe }

pub struct Particles {
    pub x: Vec<V3>, pub v: Vec<V3>,
    pub m: Vec<f64>, pub rho: Vec<f64>, pub delta: Vec<f64>, pub kind: Vec<Kind>,
}
/// Spec 1.2 derived quantities. Probes: a_eff = kappa0, m_g = 1, rest unused.
pub struct Derived {
    pub a: Vec<f64>, pub kappa: Vec<f64>, pub beta: Vec<f64>, pub tau_c: Vec<f64>,
    pub a_fun: Vec<f64>, pub a_eff: Vec<f64>, pub s: Vec<f64>, pub m_g: Vec<f64>,
    pub series_used: Vec<bool>,
}
#[derive(Clone, Debug)]
pub struct Globals {
    pub g_n: f64, pub kappa0: f64, pub xi: f64, pub v_g: f64, pub alpha_bg: f64, pub w: V3,
    pub nside0: u64, pub ell_cap: u32, pub f: f64, pub n_gl: usize, pub n_az: usize,
    /// Some(pi) = "uncalibrated mode"; None = eq. 19.
    pub pi_override: Option<f64>,
}
impl Globals {
    /// Eq. 19.
    pub fn pi_flux(&self) -> f64 {
        self.pi_override.unwrap_or(4.0 * std::f64::consts::PI * self.g_n / (self.xi * self.kappa0 * self.kappa0))
    }
    pub fn defaults() -> Globals {
        Globals { g_n: 1.0, kappa0: 2e-7, xi: 1.0, v_g: 1e3, alpha_bg: 0.0, w: [0.0; 3],
                  nside0: 8, ell_cap: 12, f: 0.15, n_gl: 32, n_az: 32, pi_override: None }
    }
}
/// Addendum II.4 validation mode.
#[derive(Clone, Copy, Debug)]
pub struct Validation { pub f_v: f64, pub f_s: f64, pub ell_cap: u32 }
impl Validation {
    pub fn default_validation() -> Self { Validation { f_v: 0.02, f_s: 4e-4, ell_cap: 14 } }
    /// Rim-refined only (f_v unchanged): no gain (1.61e-5, 3.9M leaves), the floor there is the interior rho^2 error.
    pub fn hi() -> Self { Validation { f_v: 0.02, f_s: 5e-5, ell_cap: 17 } }
    /// f_v = 0.002: MEASURED |R-0.8| = 3.32e-6 at tau = 20 (4.47M leaves, ~1 s). NOT sufficient for 1e-6.
    /// (An earlier note recorded 6.96e-7 for this preset; it does not reproduce. See ADDENDUM3.md and CONVERGENCE.md.)
    pub fn xhi() -> Self { Validation { f_v: 0.002, f_s: 4e-4, ell_cap: 14 } }
    /// Certified preset for the Spec's 1e-6 target of test (c): MEASURED |R-R_exact| = 4.3e-7 (tau 20), 1.8e-7 (tau 5),
    /// 4.7e-8 (tau 1), 64.8M leaves, ~14 s per evaluation, 0 cap hits. Error falls like leaves^-0.75 to -0.9 (the Spec's
    /// O(h^{3/2}) silhouette-edge midpoint error), so each further 4x of leaves buys only ~3x in accuracy.
    pub fn certified() -> Self { Validation { f_v: 7e-4, f_s: 4e-4, ell_cap: 15 } }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub mode: Mode, pub instrumented: bool,
    /// Force Gauss-Legendre even where closed form is allowed (test b).
    pub force_gl: bool,
    /// 0 = unlimited. Exceeding aborts with CoreError::LeafBudget (never silent coarsening).
    pub leaf_budget: u64,
    pub validation: Option<Validation>,
    /// Evaluate targets in parallel (rayon, feature "parallel"). Results are bitwise identical to serial.
    pub parallel: bool,
    /// Use the O(m^2) reference candidate graph instead of the spatial hash (equivalent; for benchmarks/tests).
    pub brute_candidates: bool,
    /// NON-CONFORMING preview path (Spec 4.4 "thin-fast"): Barnes-Hut monopole, no overlap correction.
    /// Gated: V0, alpha_bg = 0, all tau_c < 1e-2, opening angle <= 0.5.
    pub thin_fast: bool,
    pub thin_theta: f64,
}
#[derive(Debug)]
pub enum CoreError {
    InvalidParticle(usize), NonFinite { target: usize }, LeafBudget { target: usize },
    Gating(&'static str), Config(&'static str),
}

impl Particles {
    /// Validate the public structure-of-arrays boundary before any indexed physics work.
    /// Probes are point targets with unit inertial mass; density is stored but not used to derive
    /// their radius or opacity. Still require the stored per-particle mass/density to be valid.
    pub fn validate(&self) -> Result<(), CoreError> {
        let n = self.x.len();
        if self.v.len() != n || self.m.len() != n || self.rho.len() != n
            || self.delta.len() != n || self.kind.len() != n {
            return Err(CoreError::Config("particle arrays have different lengths"));
        }
        for i in 0..n {
            if !self.x[i].iter().all(|v| v.is_finite())
                || !self.v[i].iter().all(|v| v.is_finite()) {
                return Err(CoreError::InvalidParticle(i));
            }
            if !(self.m[i] > 0.0) || !(self.rho[i] > 0.0)
                || !self.m[i].is_finite() || !self.rho[i].is_finite() {
                return Err(CoreError::InvalidParticle(i));
            }
            if self.kind[i] == Kind::Body {
                if !self.delta[i].is_finite() || !(1.0 + self.delta[i] > 0.0) {
                    return Err(CoreError::InvalidParticle(i));
                }
            } else if self.m[i] != 1.0 {
                return Err(CoreError::InvalidParticle(i));
            }
        }
        Ok(())
    }
}

impl Derived {
    /// Guard force evaluation against stale or externally-constructed derived arrays.
    pub fn validate_shape(&self, n: usize) -> Result<(), CoreError> {
        let lengths = [self.a.len(), self.kappa.len(), self.beta.len(), self.tau_c.len(),
            self.a_fun.len(), self.a_eff.len(), self.s.len(), self.m_g.len(), self.series_used.len()];
        if lengths.iter().any(|&len| len != n) {
            return Err(CoreError::Config("derived arrays do not match particle count"));
        }
        for i in 0..n {
            let values = [self.a[i], self.kappa[i], self.beta[i], self.tau_c[i], self.a_fun[i],
                self.a_eff[i], self.s[i], self.m_g[i]];
            // Bodies have positive geometry/opacity; probes intentionally have zero
            // occluder geometry but retain a positive target area, S and gravitational mass.
            if values.iter().any(|v| !v.is_finite())
                || self.a[i] < 0.0 || self.kappa[i] < 0.0 || self.beta[i] < 0.0
                || self.tau_c[i] < 0.0 || self.a_fun[i] < 0.0
                || !(self.a_eff[i] > 0.0) || !(self.s[i] > 0.0) || !(self.m_g[i] > 0.0) {
                return Err(CoreError::InvalidParticle(i));
            }
        }
        Ok(())
    }
}

impl Globals {
    /// Domain checks for every public entry point. Limits are deliberately broad enough for
    /// verification presets (including the certified refinement cap at 15), while preventing
    /// invalid calibration and unsafe HEALPix shifts.
    pub fn validate(&self) -> Result<(), CoreError> {
        let finite_positive = |x: f64| x.is_finite() && x > 0.0;
        if !finite_positive(self.g_n) || !finite_positive(self.kappa0) || !finite_positive(self.xi)
            || !finite_positive(self.v_g) {
            return Err(CoreError::Config("G_N, kappa0, xi and v_g must be positive and finite"));
        }
        if !self.alpha_bg.is_finite() || self.alpha_bg < 0.0 {
            return Err(CoreError::Config("alpha_bg must be finite and >= 0"));
        }
        if !self.w.iter().all(|v| v.is_finite()) {
            return Err(CoreError::Config("flux-frame velocity W must be finite"));
        }
        let w2 = self.w[0] * self.w[0] + self.w[1] * self.w[1] + self.w[2] * self.w[2];
        if !w2.is_finite() || w2.sqrt() > 0.1 * self.v_g {
            return Err(CoreError::Config("|W| must be <= 0.1 v_g"));
        }
        if !(4..=16).contains(&self.nside0) || !self.nside0.is_power_of_two() {
            return Err(CoreError::Config("nside0 must be 4, 8 or 16"));
        }
        if self.ell_cap > 20 {
            return Err(CoreError::Config("ell_cap must be <= 20"));
        }
        if !self.f.is_finite() || !(0.0 < self.f && self.f <= 1.0) {
            return Err(CoreError::Config("overlap refinement f must be in (0, 1]"));
        }
        if !(16..=64).contains(&self.n_gl) || self.n_az < 8 || self.n_az > 1024 {
            return Err(CoreError::Config("n_gl must be 16..=64 and n_az must be 8..=1024"));
        }
        if let Some(pi) = self.pi_override {
            if !finite_positive(pi) { return Err(CoreError::Config("uncalibrated Pi must be positive and finite")); }
        }
        if !finite_positive(self.pi_flux()) {
            return Err(CoreError::Config("calibrated Pi must be positive and finite"));
        }
        Ok(())
    }
}

impl Options {
    pub fn validate(&self) -> Result<(), CoreError> {
        if let Some(v) = self.validation {
            if !v.f_v.is_finite() || !(0.0 < v.f_v && v.f_v <= 1.0)
                || !v.f_s.is_finite() || !(0.0 < v.f_s && v.f_s <= 1.0)
                || v.ell_cap > 20 {
                return Err(CoreError::Config("validation f_v/f_s must be in (0, 1] and ell_cap <= 20"));
            }
        }
        if self.thin_fast && (!self.thin_theta.is_finite() || !(0.0 < self.thin_theta && self.thin_theta <= 0.5)) {
            return Err(CoreError::Config("thin-fast opening angle must be in (0, 0.5]"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct PairMoment { pub k: u32, pub j: u32, pub p: f64, pub omega: f64, pub tau_bar: f64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct LeafRecord {
    pub k: u32, pub level: u8, pub pix: u64,
    pub d_total: f64, pub d_pair_sum: f64, pub n_touch: u16,
    pub d_eps: f64, pub d_pair_sum_eps: f64, pub n_touch_eps: u16,
}
#[derive(Default, Debug)]
pub struct Sinks { pub pairs: Vec<PairMoment>, pub leaves: Vec<LeafRecord> }
#[derive(Default, Debug, Clone)]
pub struct Status {
    pub leaf_count: u64, pub cap_hit_count: u64,
    /// max over targets of |F_corr| / |F_sh|  (F_sh = F_pair + F_corr)
    pub max_corr_over_f: f64,
    pub thin_valid: bool, pub v2_valid: bool,
    pub overlap_present: bool, pub interior_target_present: bool,
    /// True when the force came from the non-conforming thin-fast path.
    pub thin_fast: bool,
}
pub struct ForceOutput {
    pub f_pair: Vec<V3>, pub f_corr: Vec<V3>, pub f_dr: Vec<V3>, pub f_ab: Vec<V3>, pub f_total: Vec<V3>,
    pub status: Status, pub sinks: Option<Sinks>,
}
