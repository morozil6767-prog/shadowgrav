//! Simulation state (Prompt 2.3). Owns particles, derived quantities, globals, options,
//! the current force output and accelerations. The force core is only ever *called*
//! (`evaluate_forces`); nothing in force/pair/overlap/... is touched.
//!
//! Atomicity: `integrator::advance_by` snapshots x, v before a step and restores them if the
//! step fails (NaN/Inf, gating, leaf budget). `World.force`, `World.acc`, time and the ledger
//! are only replaced after a step has fully succeeded.

use crate::diagnostics::{self, Ledger};
use crate::evaluate_forces;
use crate::geom::V3;
use crate::integrator::StepStats;
use crate::pair::Quad;
use crate::sum::Neum;
use crate::types::*;

#[derive(Debug)]
pub enum SimError {
    Core(CoreError),
    NonFiniteState { index: usize },
    BadConfig(&'static str),
}
impl From<CoreError> for SimError {
    fn from(e: CoreError) -> Self {
        SimError::Core(e)
    }
}

/// Eq. 11-13 step control. `Adaptive` is flagged non-reversible / non-symplectic for the whole run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepControl {
    Fixed { h: f64 },
    Adaptive { h_max: f64, eta_t: f64 }, // eta_t = 0.005 by Contract
}

#[derive(Clone, Debug)]
pub struct SimConfig {
    pub step: StepControl,
    /// Conservation ledger is updated every `diag_every` steps (>= 1).
    pub diag_every: u64,
    /// Energy-deposition factors (Spec table); used only by the heating diagnostic.
    pub eta_e: f64,
    pub zeta: f64,
    /// V1 only: the shadow force is position-only, so the fixed-point loop re-evaluates only the
    /// closed-form eq. 8 drag. Result is identical to a full evaluation (pinned by a test).
    pub v1_shortcut: bool,
}
impl SimConfig {
    pub fn fixed(h: f64) -> SimConfig {
        SimConfig {
            step: StepControl::Fixed { h },
            diag_every: 1,
            eta_e: 1.0,
            zeta: 0.5,
            v1_shortcut: true,
        }
    }
    pub fn validate(&self) -> Result<(), SimError> {
        if self.diag_every == 0 {
            return Err(SimError::BadConfig("diag_every must be >= 1"));
        }
        if !self.eta_e.is_finite() || !(0.0..=1.0).contains(&self.eta_e) {
            return Err(SimError::BadConfig("eta_E must be finite and in [0, 1]"));
        }
        if self.zeta != 0.5 && self.zeta != 1.0 {
            return Err(SimError::BadConfig("zeta must be 0.5 or 1"));
        }
        let ok = match self.step {
            StepControl::Fixed { h } => h > 0.0 && h.is_finite(),
            StepControl::Adaptive { h_max, eta_t } => {
                h_max > 0.0 && h_max.is_finite() && eta_t > 0.0 && eta_t.is_finite()
            }
        };
        if ok {
            Ok(())
        } else {
            Err(SimError::BadConfig("invalid step control"))
        }
    }
}

/// Sticky run labels (Contract: adaptive-h flag, cap-hit flag, fixed-point flag).
#[derive(Clone, Debug, Default)]
pub struct RunFlags {
    pub adaptive_step_used: bool,
    pub fp_nonconverged_steps: u64,
    pub fp_max_residual: f64,
    pub cap_hit_total: u64,
}
impl RunFlags {
    pub fn reversible(&self) -> bool {
        !self.adaptive_step_used
    }
    pub fn symplectic_step(&self) -> bool {
        !self.adaptive_step_used
    }
}

/// a_k = F_k / m_k for bodies; probes are never integrated (acc = 0).
pub fn accelerations(p: &Particles, f: &[V3]) -> Vec<V3> {
    (0..p.x.len())
        .map(|k| {
            if p.kind[k] == Kind::Body {
                let m = p.m[k];
                [f[k][0] / m, f[k][1] / m, f[k][2] / m]
            } else {
                [0.0; 3]
            }
        })
        .collect()
}

fn checked_accelerations(p: &Particles, f: &[V3]) -> Result<Vec<V3>, SimError> {
    let a = accelerations(p, f);
    if let Some(index) = a.iter().position(|v| !v.iter().all(|x| x.is_finite())) {
        return Err(SimError::Core(CoreError::NonFinite { target: index }));
    }
    Ok(a)
}

pub struct World {
    pub p: Particles,
    pub dv: Derived,
    pub g: Globals,
    pub opt: Options,
    pub cfg: SimConfig,
    /// Force output at the current (x, v). Component buffers stay separate (f_pair/f_corr/f_dr/f_ab).
    pub force: ForceOutput,
    pub acc: Vec<V3>,
    pub step_count: u64,
    pub flags: RunFlags,
    pub ledger: Ledger,
    pub last_step: Option<StepStats>,
    /// Incremented whenever derived quantities are recomputed (m, rho, delta or kappa0 changed).
    pub derived_version: u64,
    pub(crate) time: Neum,
    pub(crate) quad: Quad,
}

fn check_lengths(p: &Particles) -> Result<(), SimError> {
    let n = p.x.len();
    if p.v.len() != n
        || p.m.len() != n
        || p.rho.len() != n
        || p.delta.len() != n
        || p.kind.len() != n
    {
        return Err(SimError::BadConfig(
            "particle arrays have different lengths",
        ));
    }
    Ok(())
}

impl World {
    pub fn new(p: Particles, g: Globals, opt: Options, cfg: SimConfig) -> Result<World, SimError> {
        check_lengths(&p)?;
        cfg.validate()?;
        let dv = crate::derive::derive(&p, &g)?; // rejects m<=0, rho<=0 (Spec 1.6)
        let force = evaluate_forces(&p, &dv, &g, &opt)?; // also enforces V2 gating
        let acc = checked_accelerations(&p, &force.f_total)?;
        let quad = Quad::new(g.n_gl);
        let flags = RunFlags {
            adaptive_step_used: matches!(cfg.step, StepControl::Adaptive { .. }),
            ..Default::default()
        };
        let mut w = World {
            p,
            dv,
            g,
            opt,
            cfg,
            force,
            acc,
            step_count: 0,
            flags,
            ledger: Ledger::default(),
            last_step: None,
            derived_version: 0,
            time: Neum::default(),
            quad,
        };
        w.validate()?;
        w.rebase_ledger();
        Ok(w)
    }

    pub fn time(&self) -> f64 {
        self.time.value()
    }
    pub fn n(&self) -> usize {
        self.p.x.len()
    }
    pub fn is_body(&self, k: usize) -> bool {
        self.p.kind[k] == Kind::Body
    }

    /// Revalidate the mutable public state before stepping. This prevents callers that edit
    /// public fields directly from bypassing constructor/live-edit checks (for example, a zero
    /// `diag_every` would otherwise panic in the conservation-ledger modulo operation).
    pub fn validate(&self) -> Result<(), SimError> {
        self.p.validate()?;
        self.g.validate()?;
        self.opt.validate()?;
        self.cfg.validate()?;
        let n = self.n();
        self.dv.validate_shape(n)?;
        if self.acc.len() != n {
            return Err(SimError::BadConfig(
                "acceleration array does not match particle count",
            ));
        }
        let arrays = [
            self.force.f_pair.len(),
            self.force.f_corr.len(),
            self.force.f_dr.len(),
            self.force.f_ab.len(),
            self.force.f_total.len(),
        ];
        if arrays.iter().any(|&len| len != n) {
            return Err(SimError::BadConfig(
                "force arrays do not match particle count",
            ));
        }
        for i in 0..n {
            let values = [
                self.acc[i][0],
                self.acc[i][1],
                self.acc[i][2],
                self.force.f_pair[i][0],
                self.force.f_pair[i][1],
                self.force.f_pair[i][2],
                self.force.f_corr[i][0],
                self.force.f_corr[i][1],
                self.force.f_corr[i][2],
                self.force.f_dr[i][0],
                self.force.f_dr[i][1],
                self.force.f_dr[i][2],
                self.force.f_ab[i][0],
                self.force.f_ab[i][1],
                self.force.f_ab[i][2],
                self.force.f_total[i][0],
                self.force.f_total[i][1],
                self.force.f_total[i][2],
            ];
            if values.iter().any(|x| !x.is_finite()) {
                return Err(SimError::Core(CoreError::NonFinite { target: i }));
            }
        }
        Ok(())
    }

    /// Switch step control. Choosing Adaptive sets the sticky non-reversible flag.
    pub fn set_step_control(&mut self, ctl: StepControl) -> Result<(), SimError> {
        let mut c = self.cfg.clone();
        c.step = ctl;
        c.validate()?;
        if matches!(ctl, StepControl::Adaptive { .. }) {
            self.flags.adaptive_step_used = true;
        }
        self.cfg = c;
        Ok(())
    }

    /// Restart the conservation baselines at the current state (after any discontinuous edit).
    pub fn rebase_ledger(&mut self) {
        let c = diagnostics::conservation(self);
        let (rs, rt) = diagnostics::r_p_pair(self);
        self.ledger = Ledger::start(&c, rs, rt);
    }

    pub fn record_conservation(&mut self) {
        let c = diagnostics::conservation(self);
        let (rs, rt) = diagnostics::r_p_pair(self);
        self.ledger.update(&c, rs, rt);
    }

    /// Re-evaluate forces / accelerations at the current state and rebase the ledger.
    pub fn refresh_forces(&mut self) -> Result<(), SimError> {
        self.p.validate()?;
        self.g.validate()?;
        self.opt.validate()?;
        self.cfg.validate()?;
        self.dv.validate_shape(self.n())?;
        let force = evaluate_forces(&self.p, &self.dv, &self.g, &self.opt)?;
        let acc = checked_accelerations(&self.p, &force.f_total)?;
        self.acc = acc;
        self.force = force;
        self.rebase_ledger();
        Ok(())
    }

    /// Replace body velocities (probes untouched). Atomic: restored on failure.
    pub fn set_velocities(&mut self, v: Vec<V3>) -> Result<(), SimError> {
        // Validate the existing structure before indexing it for the edit.
        self.p.validate()?;
        if v.len() != self.n() {
            return Err(SimError::BadConfig("velocity array length"));
        }
        let old = self.p.v.clone();
        for (k, vel) in v.iter().enumerate() {
            if self.p.kind[k] == Kind::Body {
                self.p.v[k] = *vel;
            }
        }
        match self.refresh_forces() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.p.v = old;
                Err(e)
            }
        }
    }

    /// Live global change (kappa0, xi, v_g, alpha_bg, W, f, ...). Applied between steps only.
    /// Full re-derivation + fresh a_n; ledger rebased (energy is discontinuous across the edit).
    /// Pi follows eq. 19 automatically unless `pi_override` is set (uncalibrated mode).
    pub fn apply_globals<F: FnOnce(&mut Globals)>(&mut self, f: F) -> Result<(), SimError> {
        let old = self.g.clone();
        f(&mut self.g);
        match self.try_rederive() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.g = old;
                Err(e)
            }
        }
    }

    /// Live option change (mode, force_gl, leaf_budget, validation, instrumented).
    pub fn apply_options<F: FnOnce(&mut Options)>(&mut self, f: F) -> Result<(), SimError> {
        let old = self.opt.clone();
        f(&mut self.opt);
        match self.try_rederive() {
            Ok(()) => Ok(()),
            Err(e) => {
                self.opt = old;
                Err(e)
            }
        }
    }

    fn try_rederive(&mut self) -> Result<(), SimError> {
        self.cfg.validate()?;
        let dv = crate::derive::derive(&self.p, &self.g)?;
        let force = evaluate_forces(&self.p, &dv, &self.g, &self.opt)?;
        let acc = checked_accelerations(&self.p, &force.f_total)?;
        self.acc = acc;
        self.dv = dv;
        self.force = force;
        self.quad = Quad::new(self.g.n_gl);
        self.derived_version += 1;
        self.rebase_ledger();
        Ok(())
    }

    /// Human-readable conformance labels required by the Contract.
    pub fn labels(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.flags.adaptive_step_used {
            v.push("adaptive-h: non-reversible, non-symplectic");
        }
        if self.flags.fp_nonconverged_steps > 0 {
            v.push("fixed-point (eq. 12) not converged on some steps");
        }
        if self.flags.cap_hit_total > 0 {
            v.push("cap-hit: refinement cap reached");
        }
        if self.g.pi_override.is_some() {
            v.push("uncalibrated mode (Pi overridden)");
        }
        if self.ledger.ever_nonconservative {
            v.push("energy: diagnostic, non-conservative regime");
        }
        v
    }
}
