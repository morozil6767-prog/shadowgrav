# Specification Clarification Addendum 3 (retraction of the test (c) lock, certified quadrature, quadrature structure)

Status: proposed for the frozen contract. Supersedes ADDENDUM2 item 6. Nothing here weakens a Contract prohibition.
Items 1-3 correct the record; items 4-8 are findings with consequences for the Spec text.

## Evidence legend (used here and in TRACEABILITY.md)

| tag | meaning |
|---|---|
| **[U]** | executed on the owner's machine and reported in the handoff (Rust, release) |
| **[S]** | executed by the workspace_6 sandbox session (Rust, serial, no crates.io), as reported in its change log; not re-run in the session that wrote this addendum |
| **[P]** | reproduced in Python (numpy / scipy / mpmath) in the session that wrote this addendum; scripts are in `oracle/` and `report/` |
| **[D]** | derived analytically from the Spec / the code, and checked against [S] or [P] numbers |
| **[N]** | not executed anywhere yet |

The session that wrote this addendum had **no Rust toolchain**. Every Rust statement below is therefore [U] or [S] or [N]; none is [P].

---

## 1. Retraction: test (c) was not "locked"

ADDENDUM2 item 6 and the handoff stated that `Validation::xhi()` (f_v = 0.002, f_s = 4e-4, cap 14) gives |R - 0.8| = 6.96e-7 with about 4.0M leaves
and that test (c) is therefore closed at the Spec's 1e-6.

* A fresh measurement gives **|R - 0.8| = 3.322e-6 with 4,467,284 leaves**, 0 cap hits, about 1.1 s **[S]** (`report/data/convergence.txt`, `scan_xhi.csv`).
* The two records differ in leaf count as well as in error (4.0M vs 4.47M), so they were not the same configuration run twice. Whether the 6.96e-7 was a
  transcription error or a different build or platform cannot be decided from here.
* Decisive and cheap: `cargo run --release -p shadowgrav-core --bin verify -- c` on the owner's machine prints the xhi row. If it reads 4,467,284 leaves
  and 3.32e-6, the earlier figure was a recording error. If it reads anything else, results depend on build or platform, which is a reproducibility
  finding that must be understood before any number is published (see item 8 and the CI reproducibility job).
* The independent oracle claim in the handoff **is** confirmed **[P]**: exact R(20) = 0.800000002009972, i.e. 0.8 + 2.01e-9 (mpmath, 30 digits; a scipy
  twin agrees to 4e-15). The 1e-6 tolerance is therefore a statement about quadrature, not a convention about the limit.

Consequence for the handoff text: the sentences "Test (c) is considered locked", "xhi f_v=0.002 ... 6.96e-7" and "Test (c) at 1e-6 is considered met in
validation/xhi mode" are withdrawn. The "integrator 21 passed, 2 ignored" line is also stale: `tests/integrator.rs` now holds 25 tests (23 run by default, 2 ignored): the 21 + 2 of the handoff plus the certified-mode test and the xhi regression bound (exact inventory in TRACEABILITY.md).

## 2. Test (c) conformance statement (replaces ADDENDUM2 item 6)

| mode | parameters | leaves (tau = 20) | seconds | \|R - R_exact\| at tau = 20 | worst over tau | status for Spec 7(c) |
|---|---|---|---|---|---|---|
| standard | f = 0.15, cap 12 | 1,232 | < 0.01 | 1.29e-3 | 3.3e-3 (tau = 200) | meets the **provisional** 5e-3 only |
| validation | f_v 0.02, f_s 4e-4, cap 14 | 540,512 | 0.15 | 1.58e-5 | 2.96e-5 (tau = 7.3) | not 1e-6 |
| xhi | f_v 0.002, f_s 4e-4, cap 14 | 4,467,284 | 1.1 | 3.32e-6 | 3.32e-6 (tau = 20) | not 1e-6 |
| **certified** | f_v 7e-4, f_s 4e-4, cap 15 | 64,767,596 | 14 | **4.31e-7** | 4.31e-7 (8 taus) | **meets 1e-6** |

All rows [S]; "worst over tau" is over the 64-point scans in `report/data/scan_*.csv` except certified (8 optical depths: 0.1, 0.3, 1, 3, 5, 10, 20, 50).
I re-derived the worst-over-tau column from the CSVs against the oracle **[P]**.

Statement adopted: *test (c) at the Spec's 1e-6 is satisfied by `Validation::certified()` on the three-sphere geometry at tau = 20, with margin 2.3 and 0 cap hits;
standard mode satisfies only the provisional 5e-3 clause.* Margin 2.3 on a single geometry is evidence, not a proof of 1e-6 accuracy elsewhere (item 5).

Reproduce: `cargo run --release -p shadowgrav-core --bin sgrun -- certify` (about 40 s; exit code 1 if any |R - R_exact| > 1e-6) and test
`screening_certified_mode_meets_1e6`.

## 3. Quadrature structure: leaf count is a staircase, not a continuum **[D, checked against S]**

The cell rule (Spec 4.3 item 5, validation form in `overlap.rs::cell_body`) is: a cell with at least two touching discs is a leaf at level l when
rho_l <= f_v * theta_min, and, if a silhouette lies within rho_l of its centre, also rho_l <= f_s * theta_eff(that disc). With
rho_l = 1.5 sqrt(pi/3) / (8 * 2^l) = 1.5354 / (8 * 2^l):

* **Only whole levels are possible.** For test (c) the nested pair seen from A and from C has theta_min = asin(1/20) = 0.050021. Level l is chosen for
  f_v in [rho_l / theta_min, rho_(l-1) / theta_min). The boundaries are:

| interior level l | Nside | f_v range selecting it | predicted interior leaves (2 targets) | measured leaves (tau = 20) |
|---|---|---|---|---|
| 10 | 8192 | [3.75e-3, 7.49e-3) | 1.007e6 | 1.47e6 at f_v = 5e-3 (rim cells included) |
| 11 | 16384 | [1.87e-3, 3.75e-3) | 4.03e6 | 4.47e6 at f_v = 2e-3 |
| 12 | 32768 | [9.37e-4, 1.87e-3) | 1.612e7 | 1.651e7 at f_v = 1.5e-3 and 1e-3 (identical) |
| 13 | 65536 | [4.68e-4, 9.37e-4) | 6.447e7 | 6.477e7 at f_v = 7e-4 and 5e-4 (identical) |
| 14 | 131072 | [2.34e-4, 4.68e-4) | 2.579e8 | not run |

  The predicted count is 2 Omega / A_l with Omega = 2 pi (1 - cos theta_min) = 7.859e-3 sr (two overlapping targets, A and C; B sees two disjoint discs). It matches
  the measured counts to 0.5% at level 13 and 2.4% at level 12; the excess is the rim band. (`oracle/stair_model.py`, pure Python.)
* **Consequences.** (i) f_v = 0.0015 and 0.001 are the same computation, as are 7e-4 and 5e-4 and the cap-15 and cap-16 rows; the identical rows in `convergence.txt` are
  not a bug. (ii) `certified()` at f_v = 7e-4 sits near the logarithmic middle of its stair (geometric mean of the edges 4.68e-4 and 9.37e-4 is 6.6e-4), so it is robust to the
  small changes in theta_min that other geometries bring. (iii) The next stair costs 4x: about 2.6e8 leaves, roughly one minute at the measured 4.6M leaves per second. (iv) Because f_v is relative to theta_min, a preset certified on this geometry is certified **for this geometry**; the level that
  results on another geometry follows from the same formula.
* **Rim level.** f_s = 4e-4 selects rho_l <= 2.0e-5, i.e. l = 14 (rho_14 = 1.17e-5). ell_cap = 14 is just sufficient; cap 15 is headroom that is never used (cap 16 reproduces
  the cap-15 leaf count exactly). A smaller silhouette (a more distant or smaller occluder) raises the required level by one for every halving of theta; the cap-hit flag is what protects this.

## 4. The measured order of convergence is not the Spec's O(h^(3/2)) - and a rim floor may exist

* Documents so far say the error "falls like leaves^-0.75, matching the Spec's O(h^3/2) edge error". Least-squares slopes over the nine measured stairs are -0.70 (tau = 20),
  -0.58 (tau = 5) and -0.67 (tau = 1) **[P]**, but the stair-to-stair error ratios at the last three stairs are rising toward 4: 2.4, 3.3 (tau = 20), 2.9, 3.7 (tau = 5), 3.1, 3.8 (tau = 1)
  **[P from S]**. A ratio of 4 per level is leaves^-1, i.e. O(rho^2): the interior midpoint and vector-weight error on a smooth integrand, not the silhouette O(h^(3/2)) term.
  The honest summary is *pre-asymptotic, approaching second order in the interior*. Extrapolations from a single power law (the leaf count predicted for 1e-7 spans 3e7 to 9e8 depending on tau and
  fit window) are therefore **not** to be quoted.
* In every validation row the **rim is fixed** at level 14 (f_s = 4e-4 is held constant), so the interior is the only thing that refines. Evidence that the rim error is not negligible against 4.3e-7:
  `Validation::hi()` refined the rim from level 14 to 17 at f_v = 0.02 and moved |R - 0.8| from 1.584e-5 to 1.61e-5 (source comment in `types.rs`; leaves 540k -> 3.9M), so the rim change is at least 2.6e-7 in
  magnitude, comparable to the certified error. Signs of the interior and rim errors are unknown, so this is a bound, not a decomposition.
* **Decisive experiment (about 1 minute, owner's machine):** `cargo run --release -p shadowgrav-core --bin verify -- rim` runs the 2 x 2 grid
  f_v in {0.0015 (level 12), 7e-4 (level 13)} x f_s in {4e-4 (rim 14), 1.2e-4 (rim 15)} at tau = 20. If the error depends on f_v only, the interior dominates and the next stair will give about 1e-7.
  If it depends on f_s, the rim floor (a few 1e-7) means a stronger certified preset must refine the rim as well. **Until this is run, `certified()` is
  certified at 4.3e-7 for this one geometry and nothing more.**

## 5. Spec 4.3's accuracy target against the sharp oracle **[P for the oracle, S for Rust]**

The Spec states a target of "relative force error of at most 1e-4 in the correction". The previous oracle for partial and 3D overlap (a 1.6e7-point Fibonacci grid) turns out to be better than its stated noise of
6e-5: against the new **sharp lens oracle** (`oracle/oracle_lens.py`) it deviates by 4.9e-6 to 1.7e-5 in |dF|/|F| and by 0.05% to 0.22% in Delta^NA **[P]**. The sharp oracle uses the exact identity
Corr = - integral over the lens of n D_B D_C dOmega, evaluated by nested adaptive quadrature; its two independent parametrisations agree to 1e-17 and a dense midpoint grid converges to it at the expected O(h^1.5) rate
(relative differences 2.4e-4, 1.1e-4, 3.9e-5 at N = 500, 1000, 2000) **[P]**.

Re-read against the sharp oracle (Rust numbers are the Fibonacci comparisons [S], so each carries up to 1.7e-5 / 0.22% of oracle error):

| mode | max \|dF\|/\|F\| | Delta^NA relative error | Spec 4.3 target (1e-4 on the force) |
|---|---|---|---|
| standard (f = 0.15) | 9.6e-4 | up to 8.6% | **not met** (9.6x over) |
| validation (f_v 0.02) | 3.7e-5 (true value 2e-5 .. 5.4e-5) | 0.42% (true 0.2% .. 0.64%) | met |

So: standard mode does not meet the Spec's own accuracy target on partially overlapping discs, and the Spec should say so (it is a visualisation/exploration mode with a measured error, as the UI already states);
validation mode meets it. New test file `tests/oracle_lens.rs` makes the comparison against the sharp reference [N: unexecuted] and prints the true numbers.

## 6. Erratum in the frozen Spec: the default cap and rho

Spec 4.3 says "defaults f = 0.15 and ell_cap = 12 (rho about 1e-5 rad)". With N_side,0 = 8 and rho_l = 1.5 sqrt(A_l), A_l = pi / (3 N_side^2), level 12 has N_side = 32768 and
**rho_12 = 4.68e-5 rad**; rho of about 1e-5 corresponds to level 14 (1.17e-5). Either the parenthesis or the default cap should change **[D]**. The code follows the formula, not the parenthesis. The range
"6 to 14" listed for ell_cap in the parameter table is consistent with the formula.

## 7. Tolerances recorded elsewhere, restated for the contract

* **Test (a) period** (ADDENDUM2 item 1, still awaiting acceptance): the (omega h)^2 / 24 phase error is the harmonic-oscillator value; Kepler with velocity Verlet gives 0.3394 (omega h)^2.
  The code asserts `period_rel_err <= 0.36 (omega h)^2` and, by `orbit_period_error_scales_as_h_squared`, a halving ratio in [3.6, 4.4]. `bin/verify` still asserted the old 5e-6 and
  would have reported a failure on a correct engine; it now uses the Addendum 2 bound (this workspace).
* **Oracle-test tolerances** are measurement-derived regression bounds, labelled as such in `tests/oracle_sky.rs` and `tests/oracle_lens.rs`; they are not accuracy claims.
* **Test (c) disjointness** (Delta^NA = 0 to 1e-13): enforced as `f_corr == [0.0; 3]` bitwise in `disjoint_has_empty_correction`, which implies it.
* **Test (c) "independent full-sky grid at N_side = 1024 to 1e-3"**: satisfied in substance by two independent oracles (Fibonacci grid and the lens/axisymmetric integrals), each stricter than 1e-3.

## 8. Items the Spec requires that are still not implemented or not asserted

| Spec item | state | next action |
|---|---|---|
| Section 6 item 6, tau_max (lower bound over {c_kj} and {n_c}) | **workspace_9:** `diagnostics::tau_max` written, unexecuted (item 11) | run `diag_gaps`; confirm the definition (item 11) |
| Section 6 item 8, Seeliger as a paired-run diagnostic | **workspace_9:** `diagnostics::seeliger_check` written, unexecuted (item 11); single-pair test `seeliger_single_pair` [U] | run `diag_gaps` |
| Section 7(c) "R decreases monotonically", "R -> 1, O(tau)" | now asserted in `spec_gaps.rs` (3 tests) [N] | run |
| Section 4.3 cap-hit behaviour at extreme occluder sizes | flag exists; no stress test | add a tiny-disc test |
| Bitwise cross-platform reproducibility (vendored libm) | `sgrun repro-hash` exists; no multi-platform record | CI job in `.github/workflows/ci.yml` |
| Rayon path | written, never executed against the serial result | `perf_equiv` on a multicore machine |

## 9. Sign-off requested from the Spec owner

1. Accept item 2 as the wording of test (c) conformance (certified preset; provisional 5e-3 for standard).
2. Accept ADDENDUM2 item 1 (test (a) period bound).
3. Decide item 6 (change the Spec's "rho about 1e-5 rad" or the default ell_cap).
4. Accept that the Spec 4.3 accuracy target applies to validation mode and above, with standard mode documented at its measured error.

## 10. Finding: `vendored-libm` does not make the conforming path platform-independent [D]

**Status (workspace_8): routed.** `healpix.rs` now uses `mathx::sin_cos`, `gl.rs` uses `mathx::cos`, `driver.rs` uses `mathx::cbrt/atan2/ln` (new `mathx::cos`, `mathx::ln`). In the default (std) build `sgrun repro-hash` is unchanged (combined `f2e108670c118a3d` before and after, [S]). The `vendored-libm` build was NOT compiled (libm crate unavailable in the sandbox) and no multi-platform hash record exists, so bitwise cross-platform reproducibility remains **not established** until CI compares hashes. Diagnostic-only code (`skymap.rs`) still uses platform trig. Original finding follows.

`mathx.rs` routes exp, expm1, atan2, sin_cos, cbrt, sinh, asinh through the pure-Rust libm, but the conforming force path also calls the platform `cos` and `sin`
directly in `healpix.rs::pix2vec_nest` (line 58, every quadrature cell centre) and `cos` in `gl.rs` (line 19, Gauss-Legendre node seeds). With `--features vendored-libm` those
still use the platform libm, so `sgrun repro-hash` can differ across operating systems even though the feature is on. Fix before relying on the CI reproducibility job:
route those calls through `mathx::sin_cos` / a new `mathx::cos`. Until then, treat bitwise cross-platform reproducibility as **not established**.

## 11. workspace_9: diagnostics added without a Rust toolchain **[N]**

The session that wrote this item could not compile or run anything (no `cargo`, no network). Everything below is written and read-checked only; evidence tag **N**
until `cargo test --release -p shadowgrav-core --test diag_gaps` has been run. Numbers marked [P] were reproduced in Python (scipy) in that session.

* **Spec 6 item 11, reference error wired in.** `diagnostics::reference_diag(w, n_chi, n_phi)` wraps `reference::reference_check` and returns `ReferenceDiag`
  (`outcome: Result<ReferenceCheck, String>`; `max_rel_diff()`, `reference_est_err()`, `exceeds_reference_resolution()`, `summary()`). A world outside reference scope
  (V1/V2, alpha_bg > 0, probes, interior pair, N > 50, grid too small) yields `Err(reason)` and the summary reads "not checked: ...": it is never reported as agreement.
  It is opt-in: `snapshot()` is unchanged; `snapshot_with(w, &SnapshotOptions { reference: Some((n_chi, n_phi)), tau_max: true })` fills `Snapshot.reference` / `Snapshot.tau_max`.
  Front end: `DiagView.ref_check` and `DiagView.tau_add_max` (protocol.rs), filled by `Sim::extra_diagnostics` only when the environment variable `SHADOWGRAV_EXTRA_DIAG` is set
  (not "0"), and also shown as two "Preset read-outs" rows through the existing `extras` list, so no egui code changed. The live view skips the reference for N > 8 (it is an all-rays integral).
  The UI files were edited and are **uncompiled**.
* **Spec 6 item 6, tau_max (lower bound).** `diagnostics::tau_max(w)` returns, per body k, `TauMax { lower_bound, from_centres, from_leaves, direction, samples, .. }` with
  tau_k(n) = sum_{j != k} beta_j L_j(n) (chord of eq. 16; material only, alpha_bg is not part of an additive depth; coincident centres use L = a as in Spec 1.6), maximised over
  the sample set {c_kj} (directions to the other bodies' centres) united with {n_c} (centres of the quadrature leaves, only when `Options.instrumented` recorded them).
  **The Spec text was not in the workspace**; this definition is reconstructed from the wording of item 8 above ("lower bound over {c_kj} and {n_c}") and from Traceability E7
  ("additive optical depth; lower bound"). Please confirm: (i) per target body, (ii) material only, (iii) static geometry in V2 (flagged by `static_geometry_only`), (iv) the true maximum is not attempted.
  Derived checks used by the tests: a centre ray has chord 2a, so a single exterior occluder gives tau_c; collinear occluders add (A-B-C: 2 tau_c, tau_c, 2 tau_c); every tau_k(n) <= sum_j tau_c,j.
  Cost: O(N^3) for the centre set, plus O(leaves x N) with leaf records (millions of leaves at certified settings): not run implicitly.
* **Spec 6 item 8, paired Seeliger.** `diagnostics::seeliger_check(w, alpha)` reads out F(alpha)/F(0) per ordered pair (the same `pair_p` kernel as the force, so the Gamma_k factor cancels)
  against exp(-alpha d), and per body from two full force evaluations (alpha_bg = 0 and alpha_bg = alpha, everything else identical; `w` untouched). V2 and bad alpha are refused.
  No pass/fail threshold is applied by the diagnostic. [P]: for the geometry of the test (tau_c = 1e-6, d = 100 a, alpha d = 0.05) the exact deviation from exp(-alpha d) is 1.0e-6
  (scipy quadrature of eq. 5 over the disc), so the Spec-test bound 2e-3 has a margin of 2000; do not read that margin as general: it needs a thin, distant occluder.
* **A7, mid-run non-finite abort.** `tests/diag_gaps.rs` triggers all three abort paths after construction: a NaN velocity and an overflowing drift (`SimError::NonFiniteState`), and a
  non-finite force (Pi = Inf gives `CoreError::NonFinite`, wrapped as `SimError::Core`), and checks that x, time, step counter, stored force and acceleration are bitwise unchanged and
  that the same world steps again once repaired. The abort restores the pre-step state; it does not repair a corrupted velocity.
* **Not done:** an egui widget for any of this; E8 (D-bar cross-check, still a GAP); anything about performance.
