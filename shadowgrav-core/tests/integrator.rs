//! Multi-step tests of the integrator layer. UNEXECUTED when written: run `cargo test --release`
//! first, then `cargo test --release -- --ignored` for the long ones.
use shadowgrav_core::diagnostics::*;
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::integrator::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

fn check_orbit(n_orbits: u32) {
    // Test (a): tau_c = 1e-6, V0, alpha_bg = 0, h = T/1000.
    let s = two_body_orbit(1e-6, 1000, Mode::V0, 1e3).unwrap();
    let r = run_orbit(s, n_orbits).unwrap();
    let wh2 = r.omega_h * r.omega_h;
    // Spec (a) quotes (wh)^2/24 (harmonic oscillator). For the Kepler circular orbit velocity Verlet has
    // period error 0.3394 (wh)^2 (replicated in Python at h = T/500..T/4000: 1.340e-5 at T/1000). Bound: 0.36 (wh)^2.
    assert!(r.period_rel_err <= 0.36 * r.omega_h * r.omega_h, "period err {:e}", r.period_rel_err);
    assert!(r.max_de <= wh2, "max dE/E {:e} > (wh)^2 {:e}", r.max_de, wh2);
    assert!(r.secular_de <= 0.05 * wh2, "secular drift {:e}", r.secular_de);   // threshold is ours: Spec says "no secular growth"
    assert!(r.max_dl <= 1e-12, "dL/L {:e}", r.max_dl);
    assert!(r.max_dp <= 1e-12, "dP {:e}", r.max_dp);
    assert!(r.max_rp_total <= 1e-12, "R_P {:e}", r.max_rp_total);
    assert_eq!(r.max_delta_na, 0.0);
    assert_eq!(r.label, EnergyLabel::Conserved);
}
#[test] fn orbit_five_orbits() { check_orbit(5); }
/// Richardson: halving h must cut the period error by 4 (second-order scheme).
#[test] fn orbit_period_error_scales_as_h_squared() {
    let err = |spo: u32| run_orbit(two_body_orbit(1e-6, spo, Mode::V0, 1e3).unwrap(), 5).unwrap().period_rel_err;
    let ratio = err(1000) / err(2000);
    assert!(ratio > 3.6 && ratio < 4.4, "ratio {ratio}");
}
#[test] #[ignore = "1e5 steps; run with --release -- --ignored"] fn orbit_hundred_orbits() { check_orbit(100); }

#[test] fn orbit_expected_omega_is_sqrt2_s() {
    let s = two_body_orbit(1e-6, 1000, Mode::V0, 1e3).unwrap();
    let sat = s.world.dv.s[0];
    assert!((s.omega / (2f64.sqrt() * sat) - 1.0).abs() < 1e-9);       // omega = sqrt(2) S
    assert!((sat - (1.0 - 3.75e-7)).abs() < 1e-9);
}

#[test] fn verlet_is_time_reversible_v0() {
    let mut s = two_body_orbit(1e-6, 200, Mode::V0, 1e3).unwrap();
    let w = &mut s.world;
    let (x0, v0) = (w.p.x.clone(), w.p.v.clone());
    for _ in 0..300 { advance(w).unwrap(); }
    let flipped: Vec<V3> = w.p.v.iter().map(|v| scale(v, -1.0)).collect();
    w.set_velocities(flipped).unwrap();
    for _ in 0..300 { advance(w).unwrap(); }
    for k in 0..2 { for c in 0..3 {
        assert!((w.p.x[k][c] - x0[k][c]).abs() < 1e-10, "x[{k}][{c}]");
        assert!((w.p.v[k][c] + v0[k][c]).abs() < 1e-10, "v[{k}][{c}]");
    } }
}

#[test] fn v0_uses_one_force_evaluation_per_step() {
    let mut s = two_body_orbit(1e-6, 100, Mode::V0, 1e3).unwrap();
    let st = advance(&mut s.world).unwrap();
    assert_eq!(st.force_evals, 1); assert_eq!(st.fp_iters, 0); assert!(st.fp_converged);
}

// ---- test (d1)
#[test] fn drag_decay_rate_matches_gamma() {
    let r = run_drag_decay(0.005, 5.0).unwrap();
    assert!(r.rel_err < 1e-5, "gamma fit {:e} vs ref {:e}: {:e}", r.gamma_fit, r.gamma_ref, r.rel_err);
    assert_eq!(r.nonconverged, 0);
    assert!(r.max_fp_iters <= FP_MAX_ITERS);
}
#[test] fn drag_scheme_bias_is_h_gamma_squared_over_12() {
    // Documents the integrator's own bias so (d1) failures are not misattributed to the force.
    let coarse = run_drag_decay(0.05, 5.0).unwrap();
    let expect = 0.05f64 * 0.05 / 12.0;
    assert!((coarse.gamma_fit / coarse.gamma_ref - 1.0 - expect).abs() < 2e-5, "{:e}", coarse.gamma_fit / coarse.gamma_ref - 1.0);
}
#[test] fn v1_runs_one_force_evaluation_per_step() {
    let g = Globals::defaults();
    let gamma = drag_gamma_ref(&g).unwrap();
    let mut w = drag_world(g, [1.0, 0.0, 0.0], 0.005 / gamma).unwrap();
    let st = advance(&mut w).unwrap();
    assert_eq!(st.force_evals, 1);
    assert!(st.fp_iters >= 2 && st.fp_converged);
}

// ---- test (d2)
#[test] fn frame_invariance_of_drag_evolution() {
    let dev = run_frame_invariance(0.005, 3.0).unwrap();
    assert!(dev < 1e-12, "dev {:e}", dev);
}

// ---- fixed-point behaviour
#[test] fn fixed_point_nonconvergence_is_flagged_not_hidden() {
    let g = Globals::defaults();
    let gamma = drag_gamma_ref(&g).unwrap();
    let mut w = drag_world(g, [1.0, 0.0, 0.0], 3.0 / gamma).unwrap();       // h gamma = 3: map is expanding
    let st = advance(&mut w).unwrap();
    assert!(!st.fp_converged && st.fp_iters == FP_MAX_ITERS);
    assert_eq!(w.flags.fp_nonconverged_steps, 1);
    assert!(w.labels().iter().any(|l| l.contains("not converged")));
}
#[test] fn v1_shortcut_is_bit_identical_to_full_evaluation() {
    let mut a = two_body_orbit(1e-3, 200, Mode::V1, 1e8).unwrap();         // gamma ~ 0.8, omega ~ 1.4
    let mut b = two_body_orbit(1e-3, 200, Mode::V1, 1e8).unwrap();
    b.world.cfg.v1_shortcut = false;
    for _ in 0..50 {
        let sa = advance(&mut a.world).unwrap(); let sb = advance(&mut b.world).unwrap();
        assert!(sa.fp_converged && sb.fp_converged);
        assert!(sa.force_evals == 1 && sb.force_evals == sb.fp_iters);
    }
    assert_eq!(a.world.p.x, b.world.p.x);
    assert_eq!(a.world.p.v, b.world.p.v);
}
#[test] fn energy_is_labelled_non_conservative_with_drag() {
    let s = two_body_orbit(1e-3, 200, Mode::V1, 1e8).unwrap();
    assert_eq!(conservation(&s.world).label, EnergyLabel::DiagnosticNonConservative);
    assert!(s.world.labels().iter().any(|l| l.contains("non-conservative")));
}

// ---- test (d3) at integrator level + gating
#[test] fn v2_orbit_converges_and_tracks_v1_at_large_vg() {
    // v_g = 1e10 (outside the Spec table range, not enforced): gamma*T ~ 0.035. At v_g = 1e8, gamma*T ~ 3.5,
    // the orbit collapses and the fixed point (contraction ~ h*gamma/2) cannot reach 1e-12 in 6 iterations.
    let worst = run_v2_vs_v1(1e-3, 1e10, 200).unwrap();                      // |eps| ~ 1e-10
    assert!(worst < 1e-5, "max separation difference {:e}", worst);
}
#[test] fn v2_gating_rejects_large_eps_and_leaves_no_world() {
    let mut g = Globals::defaults(); g.v_g = 10.0;
    let p = body_particles(vec![[0.0; 3], [1.0, 0.0, 0.0]], vec![[2.0, 0.0, 0.0], [0.0; 3]]);   // eps = 0.2
    let r = World::new(p, g, default_options(Mode::V2), SimConfig::fixed(1e-3));
    assert!(matches!(r, Err(SimError::Core(CoreError::Gating(_)))));
}
#[test] fn failed_step_is_atomic() {
    let mut g = Globals::defaults(); g.v_g = 100.0; g.kappa0 = kappa0_for_tau(1e-3, 1.0, RHO0);
    let p = body_particles(vec![[0.0; 3], [1.0, 0.0, 0.0]], vec![[8.0, 0.0, 0.0], [0.0; 3]]);   // eps = 0.08
    let mut w = World::new(p, g, default_options(Mode::V2), SimConfig::fixed(1.0)).unwrap();
    let (x0, v0, t0) = (w.p.x.clone(), w.p.v.clone(), w.time());
    // h = 1 drives |v| past 0.1 v_g during the fixed point or leaves the valid gate: either way an Err must restore state.
    if advance(&mut w).is_err() {
        assert_eq!(w.p.x, x0); assert_eq!(w.p.v, v0); assert_eq!(w.time(), t0); assert_eq!(w.step_count, 0);
    }
}

// ---- adaptive step (eq. 13)
#[test] fn adaptive_step_is_flagged_and_bounded() {
    let mut s = two_body_orbit(1e-6, 200, Mode::V0, 1e3).unwrap();
    let h_fixed = s.h;
    s.world.set_step_control(StepControl::Adaptive { h_max: 10.0 * h_fixed, eta_t: 0.005 }).unwrap();
    assert!(s.world.flags.adaptive_step_used && !s.world.flags.reversible() && !s.world.flags.symplectic_step());
    for _ in 0..200 {
        let st = advance(&mut s.world).unwrap();
        assert!(st.h > 0.0 && st.h <= 10.0 * h_fixed);
    }
    assert!(s.world.labels().iter().any(|l| l.contains("adaptive-h")));
}
#[test] fn adaptive_h_matches_eq13_for_two_bodies() {
    let s = two_body_orbit(1e-6, 200, Mode::V0, 1e3).unwrap();
    let w = &s.world;
    let a = w.dv.a[0]; let l = 1.0f64.max(a);                              // d = 1 > a
    let am = norm(&w.acc[0]); let vm = norm(&w.p.v[0]);
    let want = (0.005 * (l / am).sqrt()).min(0.005 * l / vm).min(1e9);
    assert!((adaptive_h(w, 1e9, 0.005) / want - 1.0).abs() < 1e-14);
}

// ---- probes and diagnostics aggregation
#[test] fn probes_are_static_and_report_effective_g() {
    let mut g = Globals::defaults(); g.kappa0 = kappa0_for_tau(1e-6, 1.0, RHO0);
    let mut p = body_particles(vec![[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.0, 100.0, 0.0]], vec![[0.0; 3]; 3]);
    p.kind[2] = Kind::Probe;
    let mut opt = default_options(Mode::V0); opt.instrumented = true;
    let mut w = World::new(p, g, opt, SimConfig::fixed(1e-3)).unwrap();
    let probe_x0 = w.p.x[2];
    for _ in 0..10 { advance(&mut w).unwrap(); }
    assert_eq!(w.p.x[2], probe_x0); assert_eq!(w.p.v[2], [0.0; 3]);
    let pg = probe_effective_g(&w);
    assert_eq!(pg.len(), 1);
    assert!((pg[0].g_eff - 1.0).abs() < 1e-3, "G_eff(r) = {}", pg[0].g_eff);          // S ~ 1, quadrupole ~ 4e-5
    let pairs = pair_effective_g(&w).unwrap();
    for pg in &pairs { if let Some(e) = pg.expected { assert!((pg.g_eff / e - 1.0).abs() < 1e-12); } }
}
#[test] fn heating_of_isolated_body_is_exact() {
    let mut g = Globals::defaults(); g.kappa0 = kappa0_for_tau(1e-6, 1.0, RHO0);
    let p = body_particles(vec![[0.0; 3]], vec![[0.0; 3]]);
    let mut opt = default_options(Mode::V0); opt.instrumented = true;
    let w = World::new(p, g, opt, SimConfig::fixed(1e-3)).unwrap();
    let sh = sky_and_heating(&w, 0).unwrap();
    assert_eq!(sh.d_bar, 0.0);
    let want = w.cfg.eta_e * w.cfg.zeta * w.g.pi_flux() * w.g.v_g * w.dv.a_eff[0];
    assert!((sh.p_heat / want - 1.0).abs() < 1e-15);
    assert_eq!(w.force.f_total[0], [0.0; 3]);                                  // N = 1: shadow force is 0
}
#[test] fn apply_globals_rederives_and_rebases() {
    let mut s = two_body_orbit(1e-6, 100, Mode::V0, 1e3).unwrap();
    let v0 = s.world.derived_version;
    let k_old = s.world.g.kappa0;
    s.world.apply_globals(|g| g.kappa0 = 2.0 * k_old).unwrap();
    assert_eq!(s.world.derived_version, v0 + 1);
    assert!((s.world.dv.tau_c[0] / (2.0 * 1e-6) - 1.0).abs() < 1e-6);          // tau_c doubles
    assert_eq!(s.world.ledger.samples, 1);                                     // rebased
    // thin-limit force is unchanged because Pi follows eq. 19 (G_eff = G_N S^2)
    let _ = advance(&mut s.world).unwrap();
}

// ---- test (c) scenario (expensive: quadrature cost unknown until first run)
#[test] #[ignore = "expensive; measure leaf counts first"]
fn screening_three_body_short_evolution() {
    let mut w = three_body_collinear(20.0, 1e-5, 20_000_000).unwrap();
    let r = screening_ratio(&w);
    assert!((r - 0.8).abs() < 5e-3, "R = {r}");                                // standard mode; 1e-6 needs validation mode
    assert!(norm(&w.force.f_total[1]) < 1e-9 * norm(&w.force.f_total[0]));
    for _ in 0..3 { advance(&mut w).unwrap(); }
    assert!(w.ledger.max_rp_shadow <= 1e-9, "R_P {:e}", w.ledger.max_rp_shadow);
    assert_eq!(w.ledger.label, EnergyLabel::DiagnosticNonConservative);        // sky-disc overlap present
}

/// Addendum 2: adaptive V1 stays inside the fixed-point convergence region (h*gamma <= 0.01).
#[test] fn adaptive_v1_fixed_point_converges_every_step() {
    let mut s = two_body_orbit(1e-3, 200, Mode::V1, 1e8).unwrap();            // gamma ~ 0.8
    s.world.set_step_control(StepControl::Adaptive { h_max: 1.0, eta_t: 0.005 }).unwrap();
    for _ in 0..300 {
        let st = advance(&mut s.world).unwrap();
        assert!(st.fp_converged, "h {:e} resid {:e}", st.h, st.fp_residual);
        assert!(st.h * 0.9 <= GAMMA_H_CAP / 0.7, "h*gamma cap violated: h {:e}", st.h);   // gamma <= ~0.9 early on
    }
    assert_eq!(s.world.flags.fp_nonconverged_steps, 0);
}

/// Test (c) (Addendum 3, supersedes Addendum 2 item 6): the Spec's 1e-6 tolerance is met by `Validation::certified()`
/// (measured |R-0.8| = 4.3e-7, 64.8M leaves, ~14 s). The earlier "xhi" claim of 6.96e-7 did not reproduce: xhi measures 3.32e-6.
#[test] fn screening_certified_mode_meets_1e6() {
    let mut w = three_body_collinear(20.0, 1e-4, 0).unwrap();
    w.apply_options(|o| o.validation = Some(Validation::certified())).unwrap();
    let r = screening_ratio(&w);
    assert!((r - 0.8).abs() <= 1e-6, "R = {r}, |R-0.8| = {:e}", (r - 0.8).abs());
    assert_eq!(w.force.status.cap_hit_count, 0);
    // F_B = 0 and F_A = -F_C by symmetry (R_P <= 1e-9 on the Spec scale)
    let scale = norm(&w.force.f_total[0]);
    assert!(norm(&w.force.f_total[1]) < 1e-9 * scale);
    assert!(norm(&add(&w.force.f_total[0], &w.force.f_total[2])) < 1e-9 * scale);
}

/// Regression bound on the cheap measurement preset: xhi (~1 s) is a 3.3e-6 quadrature, not a 1e-6 one. If this ever
/// improves, update CONVERGENCE.md; if it worsens, the quadrature changed.
#[test] fn screening_xhi_regression_bound() {
    let mut w = three_body_collinear(20.0, 1e-4, 0).unwrap();
    w.apply_options(|o| o.validation = Some(Validation::xhi())).unwrap();
    let r = screening_ratio(&w);
    let e = (r - 0.8).abs();
    assert!(e <= 4e-6 && e >= 2.5e-6, "xhi |R-0.8| = {e:e} (measured 3.32e-6)");
    assert_eq!(w.force.status.cap_hit_count, 0);
}
