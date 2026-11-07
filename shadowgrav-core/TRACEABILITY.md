# TRACEABILITY.md: Spec item -> test -> tolerance -> evidence
Every requirement of the frozen Mathematical Specification (and the Implementation Contract) is listed once, with the test that exercises it, the tolerance **asserted in code**, the strongest evidence that the test has actually been *executed*, and a verdict.
**Evidence tags** (full legend in ADDENDUM3.md): **U** executed on the owner's machine; **S** executed in the workspace_6 sandbox session (Rust, serial build, no crates.io); **P** reproduced in Python; **N** not executed. A row with two tags combines them (e.g. `U; S`). The session that wrote this table had no Rust toolchain, so it contributes only **P**.
**Verdicts**: PASS = a named test exercises the requirement at the stated tolerance and has been executed (U or S). PARTIAL = exercised indirectly, in part, or not yet executed. GAP = no test or not implemented.
A PASS with evidence S is weaker than a PASS with evidence U: it rests on a log, not on the owner's machine. The rayon path and every egui file are in the weakest class.

> **Snapshot note (2026-11-06):** This ledger records evidence tags from the earlier workspace_9 review. The follow-on
> source hardening and geometry regressions in `tests/hardening.rs` and `tests/geometry_suite.rs`, and the trail/arrow/export
> updates in the UI, are new source changes; their evidence is **N (not executed)** until the updated checkout passes on a
> Rust runner. The updated `.github/workflows/ci.yml` now includes UI build/tests on all three OSes, but that workflow still
> needs an actual Actions run.

## A. Derived quantities, calibration, edge cases

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| A1 | eq. 1-2 | a, kappa, beta, tau_c, A(tau), A_eff, S, m^g; A_eff = kappa0 m^g; S(tau) table | identities::saturation_function, aeff_identity_and_thin_force | 1e-12 (series vs closed form, A_eff identity); 6e-6 vs Spec table | U | **PASS** |
| A2 | eq. 2 series | series for small tau (code uses tau < 1: identical in exact arithmetic, deliberate) | identities::saturation_function (tau 0.3, 0.7, 0.999999; 1 - 3 tau/8 at 1e-6) | 1e-12 | U | **PASS** |
| A3 | eq. 19, eq. 7 | Pi = 4 pi G_N / (xi kappa0^2) gives G_eff = G_N S^2 (thin limit) | identities::aeff_identity_and_thin_force; integrator::orbit_expected_omega_is_sqrt2_s; probes_are_static_and_report_effective_g | 1e-12; 1e-9; 1e-3 (probe quadrupole) | U | **PASS** |
| A4 | section 5 uncalibrated mode | Pi override displays G_eff/G_N | none (UI shows an amber badge; no assertion on the value) | - | N | **GAP** |
| A5 | 1.6 N = 1 | force is 0 | identities::reciprocity_and_n1 | exact | U | **PASS** |
| A6 | 1.6 m <= 0 or rho <= 0 rejected | InvalidParticle | spec_gaps::invalid_mass_or_density_is_rejected | exact (5 cases) | N (new) | **PARTIAL** |
| A7 | 1.6 NaN/Inf aborts the step | NonFinite error, state restored | integrator::failed_step_is_atomic (atomicity); edge_cases::non_finite_positions_are_rejected (construction); **workspace_9, diag_gaps.rs:** midrun_nan_velocity_aborts_the_step_and_rolls_back, midrun_overflow_in_the_drift_aborts_the_step (both `NonFiniteState`), midrun_non_finite_force_aborts_the_step_and_rolls_back (`Core(NonFinite)` from the force path), non_finite_force_from_a_live_edit_is_rolled_back | exact (state bitwise unchanged) | U; S; **N** (diag_gaps not executed) | **PARTIAL** (PASS once diag_gaps runs) |
| A8 | 1.6 d < 1e-12 a | pair term 0 | edge_cases::coincident_centres_contribute_no_pair_force (relative change 0, finite) | 1e-12 | S | **PASS** |
| A9 | 1.4 probes | static, unit mass, never occlude | integrator::probes_are_static_and_report_effective_g | exact positions; 1e-3 G_eff | U | **PASS** |
| A10 | 2.5 V2 gating | alpha_bg = 0 and \|eps\| <= 0.1 | integrator::v2_gating_rejects_large_eps_and_leaves_no_world | exact | U | **PASS** |

## B. Force law

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| B1 | eq. 3-4, test (c) nested overlap | overlap correction on nested discs | integrator::screening_certified_mode_meets_1e6; oracle_sky::screening_curve_matches_exact_oracle; ignored: ..._certified_mode_matches_exact_oracle | 1e-6 (certified); 4e-5 (validation); 5e-3 (standard) | S | **PASS** |
| B2 | eq. 3-4, partial and 3D overlap | correction on non-nested discs | oracle_sky (3 tests, Fibonacci grid); oracle_lens (3 tests, sharp lens oracle) | standard 1.5e-3 / 13%; validation 1e-4 / 1e-2 | S (oracle_sky); N (oracle_lens) | **PARTIAL** |
| B3 | 4.3 item 2 / eq. 4 disjoint | empty correction list, Delta^NA = 0 | identities::disjoint_has_empty_correction; isolated_sphere_is_inert | exactly 0; bitwise | U | **PASS** |
| B4 | eq. 5, 6a exterior pair | F/F_thin = S^2, d-independent; closed form = Gauss-Legendre | identities::saturation_ratio_is_s_squared_and_d_independent; closed_form_vs_forced_gl; large_tau_limit | 1e-12; 1e-10 (tau up to 50); 1e-9 | U | **PASS** |
| B5 | eq. 6b interior pair | uniform-sphere field; continuity; cancellation-free pairing | identities::interior_thin_uniform_sphere_field; continuity_at_contact; interior_pair_cancellation_free; e_at_zero_eps_equals_p_chat | 1e-10 (tau = 1e-12); 1e-6; 1e-13 vs mpmath; 1e-13 | U | **PASS** |
| B6 | 2.4 thin limit, reciprocity | R^N = R^thin = 1 - S_k S_j; \|F12 + F21\| <= 1e-12 \|F\| | spec_gaps::newtonian_and_thin_residuals_equal_one_minus_s_squared; identities::reciprocity_and_n1 | 1e-12 | S; U | **PASS** |
| B7 | eq. 8 V1 drag | H(e) law, gamma | integrator::drag_decay_rate_matches_gamma; spec_gaps::drag_rate_diagnostic_matches_reference | 1e-5 (fit); 5e-7 | U; S | **PASS** |
| B8 | eq. 9, eq. 10 V2 aberration | first-order reference; zero-velocity reduction | identities::v2_matches_eq10_thin_pair; v2_zero_velocity_reduces_to_v0; integrator::v2_orbit_converges_and_tracks_v1_at_large_vg; tools/eq10_check.py (ADDENDUM2 item 4: residual/eps^2 = 0.11 .. 0.16) | 10 eps^2; 1e-13; 1e-5 | U; P (earlier) | **PARTIAL** |
| B9 | eq. 16 chord | ray-sphere intersection | no unit test; exercised by every oracle comparison | - | S | **PARTIAL** |
| B10 | eq. 17 sweep, alpha_bg > 0 | ordered ray-transfer recursion | identities::sweep_matches_alpha_zero_limit (alpha -> 0 only); seeliger_single_pair | 1e-10; 2e-3 | U | **PARTIAL** |

## C. Integration and conservation

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| C1 | eq. 11 Verlet | second order, reversible, one force evaluation per step in V0 | integrator::orbit_period_error_scales_as_h_squared; verlet_is_time_reversible_v0; v0_uses_one_force_evaluation_per_step | ratio in [3.6, 4.4]; 1e-10; exact | U | **PASS** |
| C2 | eq. 12 fixed point | contraction, cap, non-convergence flagged | integrator::fixed_point_nonconvergence_is_flagged_not_hidden; v1_runs_one_force_evaluation_per_step; v1_shortcut_is_bit_identical...; adaptive_v1_fixed_point_converges_every_step | exact flags | U | **PASS** |
| C3 | eq. 13 adaptive h | formula; flagged non-reversible | integrator::adaptive_h_matches_eq13_for_two_bodies; adaptive_step_is_flagged_and_bounded | 1e-14 | U | **PASS** |
| C4 | 3.4 conservation ledger | dE, dP, dL, R_P; labels | integrator::check_orbit (via orbit_five_orbits); energy_is_labelled_non_conservative_with_drag | dE <= (wh)^2; dL, dP, R_P <= 1e-12 | U (5 orbits) | **PASS** |
| C5 | 3.4 R_P for alpha_bg > 0 | physically nonzero | none | - | N | **GAP** |

## D. Quadrature (4.3, 4.4)

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| D1 | 4.3 item 3 HEALPix | centres, equal-area moments, rho_l = 1.5 sqrt(A) covers descendants | healpix_ref (5 tests) | 1e-14 .. 1e-11; rho margin 30% | U | **PASS** |
| D2 | 4.3 items 2, 4, 5 graph and cell rule | spatial hash == O(m^2) reference | perf_equiv::hash_candidates_equal_brute | exact set equality (40 trials, V0 and V2) | S | **PASS** |
| D3 | 4.3 accuracy target (1e-4) | relative force error in the correction | ADDENDUM3 item 5; oracle_lens::validation_mode_meets_spec_4_3_target... | 1e-4 | S (vs noisy oracle); N (sharp) | **PARTIAL** |
| D4 | 4.3 cap-hit flag | flag set when rho > f theta_min at the cap | edge_cases::cap_hit_flag_fires_when_the_cap_is_too_coarse (ell_cap = 3 must hit, default must not); certified test asserts 0 | exact | S | **PASS** |
| D5 | 4.4 reference mode | in-core brute-force all-rays mode (N <= 50) | reference::reference_forces / reference_check (exterior, V0, alpha_bg = 0); tests/reference_mode.rs (4 tests): reproduces exact R(5) within its own estimate, two-body vs closed form, judges the engine on partial/3D overlap, refuses out-of-scope worlds | own resolution estimate; validation vs reference <= 1e-4 | S (workspace_8) | **PASS** (scope as stated) |
| D6 | parallel == serial (bitwise) | rayon over targets | perf_equiv::parallel_hash_and_brute_are_bitwise_identical (serial build only so far) | bitwise | S (rayon not exercised) | **PARTIAL** |
| D7 | 4.4 thin-fast | gated, flagged, within Barnes-Hut error | perf_equiv::thin_fast_is_gated; thin_fast_matches_conforming_within_barnes_hut_error | 2e-2 vector-norm error | S | **PASS** |

## E. Diagnostics (section 6)

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| E1 | 6.1 G_eff | pair and probe | integrator::probes_are_static...; pair_effective_g loop | 1e-12 (pair) | U | **PASS** |
| E2 | 6.2 R^N, R^thin | residuals | spec_gaps::newtonian_and_thin_residuals_... | 1e-12 | S | **PASS** |
| E3 | 6.3 Delta^NA | non-additivity | oracle_sky, oracle_lens, skymap tests (support = overlap, never positive); ui oracle_presets_match_recorded_delta_na | see B2 | S | **PASS** |
| E4 | 6.3 Delta^sat | single-body saturation | none directly (derived from the same quantities as E2) | - | N | **PARTIAL** |
| E5 | 6.4 heating, eq. 20 | isolated body exact; D-bar with overlap | integrator::heating_of_isolated_body_is_exact (1e-15); no test with overlap | 1e-15 | U | **PARTIAL** |
| E6 | 6.5 drag rate | gamma reference | spec_gaps::drag_rate_diagnostic_matches_reference | 5e-7 | S | **PASS** |
| E7 | 6.6 tau-bar, tau_max | additive optical depth; lower bound | tau-bar computed (no test); **workspace_9:** diagnostics::tau_max (lower bound over {c_kj} and, when instrumented, {n_c}); diag_gaps::tau_max_exact_for_isolated_and_collinear_occluders, tau_max_respects_its_bounds_and_leaves_only_raise_it. Definition reconstructed from ADDENDUM3 item 8 (the Spec text was not in the workspace): see ADDENDUM3 item 11 | 1e-12 (exact cases); bounds sum tau_c and single tau_c | **N** (written, not executed) | **PARTIAL** |
| E8 | 6.7 D-bar | sky coverage | skymap mean vs eq. 20 shown in the UI; no automated cross-check | - | N | **GAP** |
| E9 | 6.8 Seeliger check | F(alpha)/F(0) vs exp(-alpha d) | identities::seeliger_single_pair (single pair); **workspace_9:** diagnostics::seeliger_check (per-pair kernel ratio + two full force evaluations, `w` untouched); diag_gaps::seeliger_paired_run_matches_exp_minus_alpha_d, seeliger_check_refuses_bad_input | 2e-3 (Spec-test value; [P] deviation for that geometry 1.0e-6); full-run vs pair-kernel 1e-12 | U (single pair); **N** (paired) | **PARTIAL** |
| E10 | 6.9 eta_EP | = delta to first order | spec_gaps::equivalence_principle_eta_ep_equals_delta_to_first_order | delta^2 + 1e-6 delta | S | **PASS** |
| E11 | 6.11 quadrature health | leaves, cap hits, max \|corr\|/\|F\|; brute-force error in reference mode | values computed and shown; **workspace_9:** brute-force error wired: diagnostics::reference_diag, `Snapshot.reference` (opt-in `snapshot_with`), UI `DiagView.ref_check` + an "extras" read-out row behind env `SHADOWGRAV_EXTRA_DIAG=1` (no egui widget, UI part uncompiled); diag_gaps::reference_diag_reports_the_engine_error, reference_diag_out_of_scope_is_not_a_pass, snapshot_extras_are_opt_in | 3e-4 + reference estimate (standard mode); out of scope reported as "not checked" | S (values); **N** (wiring) | **PARTIAL** |
| E12 | 6.12 validity flags | thin-valid, V2-valid, overlap, interior | gating tests; no assertion on the flag values themselves | - | U | **PARTIAL** |

## F. Test problems (section 7)

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| F1 | (a) period | T within the Verlet bound | integrator::orbit_five_orbits (5 orbits, U), orbit_hundred_orbits (ignored; S) | 0.36 (wh)^2 (Addendum 2.1; Spec text 5e-6 awaiting acceptance) | U; S (100 orbits) | **PASS** |
| F2 | (a) dE, dL, R_P, Delta^NA | \|dE/E\| <= (wh)^2, dL <= 1e-12, R_P <= 1e-12, Delta^NA = 0 | integrator::check_orbit | as stated | U; S | **PASS** |
| F3 | (b) saturation | S table; d-independence; large tau; interior; continuity; reciprocity | B4, B5, B6 | as in B | U | **PASS** |
| F4 | (c) \|R - 0.8\| <= 1e-6 at tau = 20 | quadrature accuracy | integrator::screening_certified_mode_meets_1e6; sgrun certify | 1e-6 (measured 4.3e-7) | S | **PASS** |
| F5 | (c) R -> 1 with O(tau); monotone | curve shape | spec_gaps::screening_ratio_decreases_monotonically; screening_deviation_is_linear_in_tau_at_small_tau; screening_curve_validation_mode_tracks_exact_oracle | 4e-5; slope in (0.18, 0.20) | N (new); curve shape also P | **PARTIAL** |
| F6 | (c) F_B = 0, F_A = -F_C, R_P <= 1e-9 | symmetry | certified test (1e-9 scale); screening_standard_mode_provisional | 1e-9 | U; S | **PASS** |
| F7 | (c) disjointness 1e-13; independent grid 1e-3 | disjoint list empty; oracle agreement | B3; oracles (ADDENDUM3 item 7) | exact; 1e-5 .. 4e-5 | U; S; P | **PASS** |
| F8 | (d1) drag fit | gamma to 1e-5 | integrator::drag_decay_rate_matches_gamma; drag_scheme_bias_is_h_gamma_squared_over_12 | 1e-5 | U | **PASS** |
| F9 | (d2) frame invariance | identical force | integrator::frame_invariance_of_drag_evolution | 1e-12 | U | **PASS** |
| F10 | (d3) aberration | eq. 10 to 10 eps^2 | B8 | 10 eps^2 | U | **PASS** |
| F11 | (e) Seeliger / heating / equivalence principle | see E9, E5, E10 | - | - | - | **PARTIAL** |

## G. Contract prohibitions

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| G1 | Contract: no Barnes-Hut/MC/uniform rays in a conforming run | thin-fast is the only deviation, labelled | D7; ui::sim (badge red, label); `Options.thin_fast` gate | - | S | **PASS** |
| G2 | Contract: no point masses, no Plummer, deficit form, calibration | by construction | code review only (a is derived from rho; eq. 19 in types.rs) | - | N | **PARTIAL** |

## H. Front end

| ID | Spec item | Requirement | Test / artifact | Asserted tolerance | Evidence | Verdict |
|---|---|---|---|---|---|---|
| H1 | sim thread, snapshots, commands, rollback, badge | protocol/sim behaviour | shadowgrav-ui::sim tests (6): every_preset_builds_and_steps, oracle_presets_match_recorded_delta_na, sim_thread_runs_and_publishes_green_orbit, bad_edit_is_rolled_back_and_flagged_red, sky_map_follows_target_and_focus_commands, load_preset_bumps_generation_and_pauses | as asserted | S (headless) | **PASS** |
| H2 | sky map core | projection round trip; D^NA <= 0; support = overlap; two-body has none | core skymap tests (4) | as asserted | S | **PASS** |
| H3 | egui files | app.rs, view3d.rs, skyview.rs | compiled and run by the owner at workspace_5 (presets a, b, c, badge, diagnostics, trails); later UI source edits (sky map, arrow projection, trail breaks, export) remain unexecuted in the 2026-11-06 patch | visual | U (ws5); N (current patch) | **PARTIAL** |

## Summary

60 rows: **39 PASS**, **18 PARTIAL**, **3 GAP** (D5, D4, A8 closed since workspace_7; E7 moved GAP -> PARTIAL in workspace_9, where nothing could be executed). No row became PASS in workspace_9: the new code and tests were written in a session with no Rust toolchain, so by the legend above they are evidence N until `cargo test --release -p shadowgrav-core --test diag_gaps` is run.

Rows whose PASS depends only on evidence S (not yet reproduced on the owner's machine): B1, D2, D7, E2, E3, E6, E10, F4, F6, H1, H2. Running `cargo test --release` on the owner's machine converts all of them to U.

### Gaps in priority order

1. **E11** (reference mode, D5 done). The engine and the diagnostics now carry the brute-force error (workspace_9); what is left is an egui widget for it and an executed run of the wiring tests.
2. **E8** (D-bar cross-check) is still a GAP; **E7, E9** (tau_max, paired Seeliger run) are implemented in workspace_9 but unexecuted.
3. ~~D4~~ closed in workspace_8 (edge_cases::cap_hit_flag_fires_when_the_cap_is_too_coarse).
4. **A4, C5** (uncalibrated mode, R_P with alpha_bg > 0): small tests, none written. A7 (mid-run NaN abort) and A8 are written (A8 passed in workspace_8; A7 awaits the first run of diag_gaps).
5. **D6** (rayon): run `perf_equiv` on a multicore machine; the bitwise claim has not been exercised with threads.
6. **B2, D3, F5** (new sharp-oracle and curve tests): run once; their measurements settle ADDENDUM3 item 5.


## v1.0-candidate execution note

On a release serial build of this tree, the following suites were executed green:

- healpix_ref, identities, integrator (incl. screening_certified_mode_meets_1e6)
- oracle_lens (non-ignored), oracle_sky (non-ignored), reference_mode
- spec_gaps, edge_cases (3), diag_gaps (11), perf_equiv (4)

GAP items remaining are process/CI (multi-OS repro-hash proof) and any intentional SCOPE_v1 deferrals — not silent missing physics.
