//! Edge cases listed as GAPs in TRACEABILITY.md: D4 (cap-hit flag can fire), A7 (non-finite input), A8 (coincident centres).
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn world_at(pos: &[V3], tau: f64) -> Result<World, SimError> {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let xs: Vec<V3> = pos.iter().map(|p| scale(p, a)).collect();
    World::new(body_particles(xs.clone(), vec![[0.0; 3]; xs.len()]), g, default_options(Mode::V0), SimConfig::fixed(1e-4))
}

/// D4: every certification claim assumes the cap-hit flag would have fired. Here it must: with ell_cap = 3 the finest cell
/// (rho_3 ~ 0.4 rad) is far larger than the silhouette of a disc of half-angle 0.05 rad, so cells at the cap cannot meet
/// rho <= f theta_min and the count must be positive; with the default cap on the same geometry it must be zero.
#[test] fn cap_hit_flag_fires_when_the_cap_is_too_coarse() {
    let mut w = world_at(&[[0.0; 3], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]], 5.0).unwrap();
    assert_eq!(w.force.status.cap_hit_count, 0, "default cap must not hit");
    w.apply_globals(|g| g.ell_cap = 3).unwrap();
    assert!(w.force.status.cap_hit_count > 0, "ell_cap = 3 must set the cap-hit flag, got {}", w.force.status.cap_hit_count);
}

/// A7: non-finite input is rejected at construction, never propagated.
#[test] fn non_finite_positions_are_rejected() {
    for bad in [f64::NAN, f64::INFINITY] {
        assert!(world_at(&[[0.0; 3], [bad, 0.0, 0.0]], 1.0).is_err(), "{bad}");
    }
}

/// A8: coincident centres (d < 1e-12 a): the pair term between them must vanish (Spec 1.6) and nothing may become non-finite.
/// Isolated by comparison: body 0's pair force with {coincident body 1, body 2 at 10a} must equal its pair force with body 2 alone.
#[test] fn coincident_centres_contribute_no_pair_force() {
    let with = match world_at(&[[0.0; 3], [0.0; 3], [10.0, 0.0, 0.0]], 1.0) {
        Ok(w) => w,
        Err(e) => { eprintln!("[edge] coincident centres rejected with {e:?} (explicit, acceptable)"); return; }
    };
    let alone = world_at(&[[0.0; 3], [10.0, 0.0, 0.0]], 1.0).unwrap();
    for f in with.force.f_total.iter().chain(with.force.f_pair.iter()) { assert!(f.iter().all(|x| x.is_finite()), "{f:?}"); }
    let diff = norm(&sub(&with.force.f_pair[0], &alone.force.f_pair[0])) / norm(&alone.force.f_pair[0]);
    eprintln!("[edge] coincident pair: relative change of body 0's pair force = {diff:e}");
    assert!(diff <= 1e-12, "coincident body contributed {diff:e}");
}
