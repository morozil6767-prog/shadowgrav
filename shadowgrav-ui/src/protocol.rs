//! The ONLY types that cross the thread boundary between the simulation thread and the UI.
//!
//!   UI  --Command-->  sim thread        (mpsc channel, applied between steps)
//!   UI  <--`Arc<FrameSnapshot>`--  sim thread   (latest-wins slot; immutable once published)
//!
//! Nothing here references `World`, so the UI cannot reach physics state (Architecture section 2, rule 4).
use shadowgrav_core::skymap::{SkyFocus, SkyMap};
use shadowgrav_core::Mode;
use std::sync::Arc;

/// Interactive quadrature tier. Maps onto Options.validation (None = Standard).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuadTier {
    /// f = 0.15 — provisional test (c) tolerance 5e-3.
    Standard,
    /// f_v = 0.02 — dynamics-grade; test (c) ~1e-5, not Spec 1e-6.
    Validation,
    /// f_v = 0.002 — finer interactive; test (c) ~3e-6, still short of Spec 1e-6.
    Fine,
    /// f_v = 7e-4, ~65M leaves — meets Spec 1e-6 on test (c) geometry; slow.
    Certified,
}

impl QuadTier {
    pub const ALL: [QuadTier; 4] = [
        QuadTier::Standard,
        QuadTier::Validation,
        QuadTier::Fine,
        QuadTier::Certified,
    ];
    pub fn label(self) -> &'static str {
        match self {
            QuadTier::Standard => "Standard",
            QuadTier::Validation => "Validation",
            QuadTier::Fine => "Fine",
            QuadTier::Certified => "Certified",
        }
    }
    pub fn detail(self) -> &'static str {
        match self {
            QuadTier::Standard => {
                "Fast. Test (c): |R-0.8| <= 5e-3 (provisional). Partial overlap coarse."
            }
            QuadTier::Validation => {
                "~0.5M leaves/eval. Dynamics-grade. Test (c) ~1e-5 — not Spec 1e-6."
            }
            QuadTier::Fine => "~4.5M leaves/eval. Test (c) ~3e-6 — still short of Spec 1e-6.",
            QuadTier::Certified => {
                "~65M leaves, ~10-20s/eval. Meets Spec 1e-6 on test (c). Prefer single evaluations."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PresetKind {
    TwoBodyOrbit,
    TwoBodyOrbitV2,
    Saturation,
    Screening,
    PartialOverlap,
    Overlap3D,
    Drag,
    DragFrame,
    DenseRandom,
}

impl PresetKind {
    pub const ALL: [PresetKind; 9] = [
        PresetKind::TwoBodyOrbit,
        PresetKind::TwoBodyOrbitV2,
        PresetKind::Saturation,
        PresetKind::Screening,
        PresetKind::PartialOverlap,
        PresetKind::Overlap3D,
        PresetKind::Drag,
        PresetKind::DragFrame,
        PresetKind::DenseRandom,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            PresetKind::TwoBodyOrbit => "(a) Two-body circular orbit (V0)",
            PresetKind::TwoBodyOrbitV2 => "(d3) Two-body orbit, V2 aberration",
            PresetKind::Saturation => "(b) Two-body saturation, d = 10a",
            PresetKind::Screening => "(c) Three-body screening (collinear)",
            PresetKind::PartialOverlap => "(c2) Partial overlap, planar (oracle-checked)",
            PresetKind::Overlap3D => "(c3) Partial overlap, 3D (oracle-checked)",
            PresetKind::Drag => "(d1) V1 drag decay",
            PresetKind::DragFrame => "(d2) Drag, flux frame W = -V0",
            PresetKind::DenseRandom => "Dense random N (performance)",
        }
    }
    pub fn blurb(&self) -> &'static str {
        match self {
            PresetKind::TwoBodyOrbit => "Thin-limit orbit. Expect T = 2pi/omega, bounded dE/E ~ (omega h)^2, dL/L ~ 1e-12.",
            PresetKind::TwoBodyOrbitV2 => "Same orbit with V2; v_g = 1e10 so gamma*T is small (outside the Spec v_g range).",
            PresetKind::Saturation => "Bodies at rest. Read G_eff vs expected G S_k S_j in the pair table. Paused: they fall.",
            PresetKind::Screening => "Collinear test (c), tau_c=20. Use Quad tier: Standard<=5e-3, Fine~3e-6, Certified<=1e-6. Paused.",
            PresetKind::PartialOverlap => "Non-nested planar overlap (oracle-checked). Prefer Validation/Fine tier. Paused.",
            PresetKind::Overlap3D => "As (c2) but the third body is displaced out of the plane: a genuinely 3D overlap geometry, also oracle-checked. Paused.",
            PresetKind::Drag => "Single body, W = 0. Expect u(t) = V0 exp(-gamma t).",
            PresetKind::DragFrame => "Body at rest in a flux frame moving at -V0: identical drag to (d1).",
            PresetKind::DenseRandom => "N random bodies at rest in a unit cube (overlap-heavy). Watch steps/s; try parallel and thin-fast.",
        }
    }
    /// What to look at in this preset (shown in the Scene panel). Statements mirror the preset constructors and tests.
    pub fn look_for(&self) -> &'static [&'static str] {
        match self {
            PresetKind::TwoBodyOrbit => &[
                "Preset read-outs: orbit period and omega against the expected values.",
                "Conservation: dE/E0 stays bounded (~(omega h)^2), dL/L0 near round-off.",
                "Sky map: one small disc, D^NA = 0 exactly (nothing overlaps). Zoom is chosen automatically.",
            ],
            PresetKind::TwoBodyOrbitV2 => &[
                "Mode V2 adds aberration: enable the 'aberr' arrow to see F_ab (tiny against F_pair).",
                "Badge is amber by design (V2 caveat); dE is a diagnostic here, not a conserved quantity.",
                "The sky map shows the static geometry only; the V2 shifted discs are not drawn.",
            ],
            PresetKind::Saturation => &[
                "Pair table: G_eff against the expected G S_k S_j (saturation lowers G_eff at tau_c ~ 1).",
                "Bodies start at rest: pressing Play lets them fall, so read the table while paused.",
                "Sky map: the disc is large and its deficit is far from linear in tau.",
            ],
            PresetKind::Screening => &[
                "Read-outs: R, |R-0.8|, quad tier, tolerance, regime status (WITHIN/OUTSIDE).",
                "Pick Quad tier: Standard<=5e-3, Validation~1e-5, Fine~3e-6, Certified<=1e-6 (slow).",
                "Sky map: select body A — D^NA only in the nested overlap.",
            ],
            PresetKind::PartialOverlap | PresetKind::Overlap3D => &[
                "Non-nested overlap: Delta^NA (body A) vs sky oracle + regime status.",
                "Prefer Validation/Fine/Certified tiers; Standard is intentionally coarse here.",
                "Select body A — D^NA support should be the lens-shaped overlap.",
            ],
            PresetKind::Drag => &[
                "Single body: u(t) should follow V0 exp(-gamma t); read u/u_expected - 1.",
                "Enable the 'drag' arrow. The sky map is empty (no occluders).",
            ],
            PresetKind::DragFrame => &[
                "Same drag as (d1) from rest in a flux frame moving at -V0; the two runs should agree.",
                "Enable the 'drag' arrow. The sky map is empty (no occluders).",
            ],
            PresetKind::DenseRandom => &[
                "Overlap-heavy: watch steps/s and the leaf count; try parallel and thin-fast (non-conforming).",
                "The live sky map is refused above the cost budget; lower N or select fewer bodies to see it.",
            ],
        }
    }
    pub fn uses_tau(&self) -> bool {
        !matches!(self, PresetKind::Drag | PresetKind::DragFrame)
    }
    pub fn uses_n(&self) -> bool {
        matches!(self, PresetKind::DenseRandom)
    }
    pub fn uses_steps(&self) -> bool {
        matches!(self, PresetKind::TwoBodyOrbit | PresetKind::TwoBodyOrbitV2)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PresetSpec {
    pub kind: PresetKind,
    pub tau_c: f64,
    pub steps_per_orbit: u32,
    pub n: u32,
}
impl PresetSpec {
    pub fn default_for(kind: PresetKind) -> PresetSpec {
        let tau_c = match kind {
            PresetKind::TwoBodyOrbit => 1e-6,
            PresetKind::TwoBodyOrbitV2 => 1e-3,
            PresetKind::Saturation => 1.0,
            PresetKind::Screening => 20.0,
            PresetKind::PartialOverlap | PresetKind::Overlap3D => 5.0,
            _ => 1e-3,
        };
        PresetSpec {
            kind,
            tau_c,
            steps_per_orbit: 1000,
            n: 100,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Play,
    Pause,
    Step(u32),
    Load(PresetSpec),
    /// Target steps per wall-clock second; 0 = as fast as the 8 ms/frame budget allows.
    SetRate(f64),
    SetFixedH(f64),
    SetAdaptive(bool),
    SetMode(Mode),
    SetKappa0(f64),
    SetXi(f64),
    SetVg(f64),
    SetAlphaBg(f64),
    SetF(f64),
    SetQuadTier(QuadTier),
    SetInstrumented(bool),
    SetLeafBudget(u64),
    /// Rayon over targets (bitwise-identical results).
    SetParallel(bool),
    /// NON-CONFORMING Barnes-Hut preview (gated in the core: V0, alpha_bg = 0, all tau_c < 1e-2).
    SetThinFast(bool),
    /// Sky-map target body (diagnostic only; never affects the physics).
    SetSkyTarget(usize),
    /// Sky-map framing: all-sky, or zoom on occluder j with half field of view = fov x (angular radius of disc j).
    SetSkyFocus(SkyFocus, f64),
    /// Compute the sky map at all (it costs sim-thread time at every diagnostics refresh).
    SetSkyEnabled(bool),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Badge {
    Green,
    Amber,
    Red,
}

#[derive(Clone, Debug)]
pub struct BodyView {
    pub x: [f64; 3],
    pub v: [f64; 3],
    pub mass: f64,
    pub radius: f64,
    pub tau_c: f64,
    pub s: f64,
    pub is_probe: bool,
    pub f_total: [f64; 3],
    pub f_pair: [f64; 3],
    pub f_corr: [f64; 3],
    pub f_dr: [f64; 3],
    pub f_ab: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct ParamView {
    pub mode: Mode,
    pub kappa0: f64,
    pub xi: f64,
    pub v_g: f64,
    pub alpha_bg: f64,
    pub g_n: f64,
    pub pi_flux: f64,
    pub calibrated: bool,
    pub h: f64,
    pub adaptive: bool,
    pub f: f64,
    pub quad_tier: QuadTier,
    pub instrumented: bool,
    pub leaf_budget: u64,
    pub ell_cap: u32,
    pub thin_fast: bool,
    pub parallel: bool,
}

#[derive(Clone, Debug)]
pub struct TargetRow {
    pub k: usize,
    pub delta_na: f64,
    pub delta_sat: f64,
    pub r_newton: f64,
    pub r_thin: f64,
    pub gamma: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct PairRow {
    pub k: usize,
    pub j: usize,
    pub g_eff: f64,
    pub expected: Option<f64>,
}

/// Spec 6 item 11 read-out carried to the front end (workspace_9). `max_rel_diff` / `est_err` are None when the world is outside
/// reference scope; `text` always says what happened. Not displayed by an egui widget yet: it also appears as an "extras" row.
#[derive(Clone, Debug)]
pub struct RefView {
    pub max_rel_diff: Option<f64>,
    pub est_err: Option<f64>,
    pub exceeds_est: Option<bool>,
    pub text: String,
}

/// Slow diagnostics (rebuilt at <= 10 Hz and shared by Arc between frames).
#[derive(Clone, Debug, Default)]
pub struct DiagView {
    pub seq: u64,
    pub e: f64,
    pub ke: f64,
    pub pe: f64,
    pub energy_nonconservative: bool,
    pub de: f64,
    pub max_de: f64,
    pub dp: f64,
    pub max_dp: f64,
    pub dl: f64,
    pub max_dl: f64,
    pub rp_shadow: f64,
    pub rp_total: f64,
    pub max_rp_total: f64,
    pub targets: Vec<TargetRow>,
    pub pairs: Vec<PairRow>,
    pub leaf_count: u64,
    pub cap_hits: u64,
    pub max_corr_over_f: f64,
    pub fp_iters: u32,
    pub fp_residual: f64,
    pub fp_converged: bool,
    pub force_evals: u32,
    pub tau_min: f64,
    pub tau_mean: f64,
    pub tau_max: f64,
    pub s_min: f64,
    pub s_mean: f64,
    pub thin_valid: bool,
    pub v2_valid: bool,
    pub overlap_present: bool,
    pub interior_present: bool,
    pub labels: Vec<String>,
    /// Preset-specific read-outs, e.g. ("R = |F_A|/|F_pair|", "0.800001").
    pub extras: Vec<(String, String)>,
    /// Sky deficit map for the selected body (None: disabled or refused, see `sky_note`).
    pub sky: Option<SkyMap>,
    pub sky_note: Option<String>,
    /// D-bar of the sky-map target from the instrumented sinks (eq. 20), for comparison with the map mean.
    pub d_bar_eq20: Option<f64>,
    /// Opt-in extra diagnostics (env SHADOWGRAV_EXTRA_DIAG=1; None otherwise = not computed, NOT "fine").
    /// Spec 6 item 11: engine-vs-all-rays reference error.
    pub ref_check: Option<RefView>,
    /// Spec 6 item 6: largest lower bound of the additive optical depth tau_max over all bodies (additive material only).
    pub tau_add_max: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct FrameSnapshot {
    pub seq: u64,
    pub load_gen: u64,
    pub preset: Option<PresetSpec>,
    pub running: bool,
    pub rate: f64,
    pub steps_per_sec: f64,
    pub time: f64,
    pub step: u64,
    pub bodies: Vec<BodyView>,
    pub params: ParamView,
    pub diag: Arc<DiagView>,
    pub badge: Badge,
    pub badge_reasons: Vec<String>,
    /// The first `badge_n_red` reasons are red (non-conforming); the rest are amber caveats.
    pub badge_n_red: usize,
    pub error: Option<String>,
}
