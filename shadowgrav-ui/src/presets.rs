//! Standard test problems (Spec section 7) as World constructors. Pure set-up: no stepping, no UI.
use crate::protocol::{PresetKind, PresetSpec};
use shadowgrav_core::driver::*;
use shadowgrav_core::state::{SimConfig, SimError, World};
use shadowgrav_core::{Globals, Mode};

/// Quantities the read-outs compare against (all derived by the core's own driver helpers).
#[derive(Clone, Copy, Debug, Default)]
pub struct Meta {
    pub period: f64,
    pub omega: f64,
    pub gamma_ref: f64,
    pub u0: f64,
}

pub struct Built {
    pub world: World,
    pub meta: Meta,
}

pub fn build(spec: &PresetSpec) -> Result<Built, SimError> {
    let mut built = match spec.kind {
        PresetKind::TwoBodyOrbit => orbit(spec, Mode::V0, 1e3)?,
        PresetKind::TwoBodyOrbitV2 => orbit(spec, Mode::V2, 1e10)?,
        PresetKind::Saturation => {
            let (w, _a) = pair_at_rest(spec.tau_c)?;
            Built {
                world: w,
                meta: Meta::default(),
            }
        }
        PresetKind::Screening => {
            let w = three_body_collinear(spec.tau_c, 1e-4, 0)?;
            Built {
                world: w,
                meta: Meta::default(),
            }
        }
        PresetKind::PartialOverlap => three_body(spec.tau_c, [12.0, 2.0, 0.0])?,
        PresetKind::Overlap3D => three_body(spec.tau_c, [12.0, 1.4, 1.4])?,
        PresetKind::Drag | PresetKind::DragFrame => drag(spec.kind == PresetKind::DragFrame)?,
        PresetKind::DenseRandom => dense_random(spec)?,
    };
    // Instrumented evaluation (pair moments, G_eff, heating) is cheap for small N.
    let small = built.world.n() <= 16;
    built.world.apply_options(|o| o.instrumented = small)?;
    Ok(built)
}

fn orbit(spec: &PresetSpec, mode: Mode, v_g: f64) -> Result<Built, SimError> {
    let s = two_body_orbit(spec.tau_c, spec.steps_per_orbit, mode, v_g)?;
    let meta = Meta {
        period: s.period,
        omega: s.omega,
        ..Meta::default()
    };
    Ok(Built {
        world: s.world,
        meta,
    })
}

/// Two equal bodies at rest, d = 10 a (test (b) geometry).
fn pair_at_rest(tau_c: f64) -> Result<(World, f64), SimError> {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau_c, 1.0, RHO0);
    let p = body_particles(
        vec![[-5.0 * a, 0.0, 0.0], [5.0 * a, 0.0, 0.0]],
        vec![[0.0; 3]; 2],
    );
    let w = World::new(p, g, default_options(Mode::V0), SimConfig::fixed(1e-4))?;
    Ok((w, a))
}

/// Oracle geometry (oracle/oracle_sky.py): bodies at 0, (10, 0, 0) and `third` (all in units of a), at rest.
fn three_body(tau_c: f64, third: [f64; 3]) -> Result<Built, SimError> {
    use shadowgrav_core::geom::scale;
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau_c, 1.0, RHO0);
    let xs: Vec<[f64; 3]> = [[0.0; 3], [10.0, 0.0, 0.0], third]
        .iter()
        .map(|p| scale(p, a))
        .collect();
    let world = World::new(
        body_particles(xs, vec![[0.0; 3]; 3]),
        g,
        default_options(Mode::V0),
        SimConfig::fixed(1e-4),
    )?;
    Ok(Built {
        world,
        meta: Meta::default(),
    })
}

/// Delta^NA of body 0 from the independent brute-force sky oracle (oracle/sky_out.txt), where recorded.
pub fn oracle_delta_na(kind: PresetKind, tau_c: f64) -> Option<f64> {
    let t = |x: f64| (tau_c - x).abs() < 1e-12;
    match kind {
        PresetKind::PartialOverlap if t(1.0) => Some(0.00287756191443983),
        PresetKind::PartialOverlap if t(5.0) => Some(0.011292426795888624),
        PresetKind::Overlap3D if t(1.0) => Some(0.003550047035907218),
        PresetKind::Overlap3D if t(5.0) => Some(0.013393782411369088),
        _ => None,
    }
}

/// (d1) body moving at V0 = v_g/1e3 in W = 0; (d2) same drag from rest with W = -V0. h*gamma = 0.005.
fn drag(frame: bool) -> Result<Built, SimError> {
    let mut g = Globals::defaults();
    let gamma_ref = drag_gamma_ref(&g)?;
    let u0 = g.v_g / 1e3;
    let h = 0.005 / gamma_ref;
    let v0 = if frame {
        g.w = [-u0, 0.0, 0.0];
        [0.0; 3]
    } else {
        [u0, 0.0, 0.0]
    };
    let world = drag_world(g, v0, h)?;
    Ok(Built {
        world,
        meta: Meta {
            gamma_ref,
            u0,
            ..Meta::default()
        },
    })
}

/// N spheres (a = 0.01) uniformly random in the unit cube, centres >= 2a apart, at rest. Deterministic (fixed seed).
fn dense_random(spec: &PresetSpec) -> Result<Built, SimError> {
    use shadowgrav_core::geom::{norm, sub};
    let a = sphere_radius(1.0, RHO0);
    let n = spec.n.max(2) as usize;
    let mut s: u64 = 0x2545F4914F6CDD1D;
    let mut u = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut xs: Vec<[f64; 3]> = Vec::with_capacity(n);
    while xs.len() < n {
        let c = [u(), u(), u()];
        if xs.iter().all(|x| norm(&sub(x, &c)) > 2.0 * a) {
            xs.push(c);
        }
    }
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(spec.tau_c, 1.0, RHO0);
    let world = World::new(
        body_particles(xs, vec![[0.0; 3]; n]),
        g,
        default_options(Mode::V0),
        SimConfig::fixed(1e-4),
    )?;
    Ok(Built {
        world,
        meta: Meta::default(),
    })
}
