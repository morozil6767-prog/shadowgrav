//! Public-API boundary hardening: malformed arrays and invalid physical parameters fail cleanly.
use shadowgrav_core::driver::{body_particles, default_options, RHO0};
use shadowgrav_core::state::{SimConfig, World};
use shadowgrav_core::{advance, advance_by, derive, evaluate_forces, CoreError, Globals, Mode};

fn one_body() -> shadowgrav_core::Particles {
    body_particles(vec![[0.0, 0.0, 0.0]], vec![[0.0, 0.0, 0.0]])
}

#[test]
fn derive_rejects_mismatched_particle_arrays_without_panicking() {
    let mut p = one_body();
    p.v.clear();
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::Config(_))));

    let mut p = one_body();
    p.delta.clear();
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::Config(_))));
}

#[test]
fn force_boundary_rejects_stale_derived_array_lengths() {
    let p = one_body();
    let g = Globals::defaults();
    let mut dv = derive(&p, &g).expect("valid derived state");
    dv.beta.clear();
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &default_options(Mode::V0)),
        Err(CoreError::Config(_))
    ));
}

#[test]
fn singleton_runs_still_reject_invalid_global_parameters() {
    for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let mut g = Globals::defaults();
        g.kappa0 = bad;
        assert!(World::new(one_body(), g, default_options(Mode::V0), SimConfig::fixed(1e-3)).is_err());

        let mut g = Globals::defaults();
        g.xi = bad;
        assert!(World::new(one_body(), g, default_options(Mode::V0), SimConfig::fixed(1e-3)).is_err());
    }
}

#[test]
fn invalid_edits_are_rejected_and_rolled_back() {
    let mut w = World::new(one_body(), Globals::defaults(), default_options(Mode::V0), SimConfig::fixed(1e-3)).unwrap();
    let old_kappa = w.g.kappa0;
    assert!(w.apply_globals(|g| g.kappa0 = f64::NAN).is_err());
    assert_eq!(w.g.kappa0, old_kappa);

    assert!(w.apply_globals(|g| g.w = [f64::INFINITY, 0.0, 0.0]).is_err());
    assert_eq!(w.g.kappa0, old_kappa);
}

#[test]
fn body_species_offset_must_keep_extinction_positive() {
    let mut p = one_body();
    p.delta[0] = -1.0;
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::InvalidParticle(0))));
}

#[test]
fn body_nonfinite_velocity_is_rejected_even_when_force_is_zero() {
    let mut p = one_body();
    p.v[0][1] = f64::NAN;
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::InvalidParticle(0))));
}

#[test]
fn documented_density_is_accepted() {
    let p = body_particles(vec![[0.0; 3]], vec![[0.0; 3]]);
    assert!(p.rho[0] > 0.0 && p.rho[0].is_finite());
    assert!(derive(&p, &Globals::defaults()).is_ok());
    assert!((p.rho[0] / RHO0 - 1.0).abs() < 1e-12);
}

#[test]
fn force_boundary_rejects_nonphysical_derived_values() {
    let p = one_body();
    let g = Globals::defaults();
    let mut dv = derive(&p, &g).expect("valid derived state");
    dv.beta[0] = -1.0;
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &default_options(Mode::V0)),
        Err(CoreError::InvalidParticle(0))
    ));

    let mut dv = derive(&p, &g).expect("valid derived state");
    dv.a_eff[0] = 0.0;
    assert!(matches!(
        evaluate_forces(&p, &dv, &g, &default_options(Mode::V0)),
        Err(CoreError::InvalidParticle(0))
    ));
}

#[test]
fn heating_parameters_follow_the_documented_domain() {
    let p = one_body();
    let g = Globals::defaults();
    for eta_e in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        let mut cfg = SimConfig::fixed(1e-3);
        cfg.eta_e = eta_e;
        assert!(World::new(particles_clone(&p), g.clone(), default_options(Mode::V0), cfg).is_err());
    }
    let mut cfg = SimConfig::fixed(1e-3);
    cfg.zeta = 0.75;
    assert!(World::new(p, g, default_options(Mode::V0), cfg).is_err());
}

fn particles_clone(p: &shadowgrav_core::Particles) -> shadowgrav_core::Particles {
    shadowgrav_core::Particles {
        x: p.x.clone(), v: p.v.clone(), m: p.m.clone(), rho: p.rho.clone(),
        delta: p.delta.clone(), kind: p.kind.clone(),
    }
}

#[test]
fn stepping_revalidates_publicly_mutable_world_configuration() {
    let mut w = World::new(one_body(), Globals::defaults(), default_options(Mode::V0), SimConfig::fixed(1e-3)).unwrap();
    let x_before = w.p.x.clone();
    w.cfg.diag_every = 0;
    assert!(advance(&mut w).is_err());
    assert_eq!(w.p.x, x_before, "invalid config must not mutate state");
    assert_eq!(w.step_count, 0);

    let mut w = World::new(one_body(), Globals::defaults(), default_options(Mode::V0), SimConfig::fixed(1e-3)).unwrap();
    w.acc.clear();
    assert!(advance_by(&mut w, 1e-3).is_err());
    assert_eq!(w.step_count, 0);
}

#[test]
fn velocity_edit_rejects_corrupt_particle_storage_without_panicking() {
    let mut w = World::new(one_body(), Globals::defaults(), default_options(Mode::V0), SimConfig::fixed(1e-3)).unwrap();
    w.p.v.clear();
    assert!(w.set_velocities(vec![[0.0; 3]]).is_err());
    assert!(w.p.v.is_empty(), "failed edit must not mutate the malformed storage");
}

#[test]
fn refresh_rejects_invalid_configuration_before_force_rebuild() {
    let mut w = World::new(one_body(), Globals::defaults(), default_options(Mode::V0), SimConfig::fixed(1e-3)).unwrap();
    w.g.alpha_bg = f64::NAN;
    assert!(w.refresh_forces().is_err());
}

#[test]
fn probes_must_keep_the_contract_unit_mass_and_valid_stored_density() {
    let mut p = one_body();
    p.kind[0] = shadowgrav_core::Kind::Probe;
    assert!(derive(&p, &Globals::defaults()).is_ok());

    p.m[0] = 2.0;
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::InvalidParticle(0))));
    p.m[0] = 1.0;
    p.rho[0] = f64::NAN;
    assert!(matches!(derive(&p, &Globals::defaults()), Err(CoreError::InvalidParticle(0))));
}
