# WS6: independent validation

Status: partly done. Done: an independent second evaluation of the force law on 58 random geometries with a disagreement table, std-only
property tests, and fuzz-style robustness tests. Not done (not possible in this environment): coverage-guided fuzzing (cargo-fuzz),
`proptest`, sanitizers (nightly), and a second implementation written from the frozen Spec PDF alone.

## 1. What the second implementation is, and what it is not

`oracle/xcheck_random.py` evaluates F_k = Gamma_k Int n D_k(n) dOmega, D_k = -expm1(-sum_j beta_j L_j(n)) (mode V0, alpha_bg = 0) by a
different method in a different language from the engine:

* No HEALPix, no overlap graph, no touching sets, no closed-form pair term. Each occluder disc is integrated with Gauss-Legendre in
  sin-substituted polar coordinates (sin psi = sin theta sin chi, dOmega = s^2 sin chi cos chi / cos psi dchi dphi). Every direction is
  counted once by chord-weight sharing, w_j = beta_j L_j / sum(beta L). A target inside a sphere uses whole-sphere sampling.
* The pair force is the direct 1-D integral (scipy `quad`), not the closed form pi s^2 A(tau).
* a, beta, tau_c, A_eff are recomputed from eqs. 1-2.
* The oracle's own error is estimated from a doubled grid (|fine - coarse| / gross) and reported with every row.

What it is NOT: it encodes the same equations as the engine, as written in the repository (`reference.rs`, `derive.rs`). The frozen Spec
PDF was not available in this session, so this is not "an implementation from the spec alone". Agreement shows the engine evaluates this law
accurately. It does not test the law, the V1/V2 modes, or the dynamics.

## 2. Results (seed 20261107; 58 geometries; 3 to 5 bodies; tau_c(0) in {0.5, 2, 5, 20}; equal and unequal masses and densities)

Scale: discrepancy / gross, gross = Int D dOmega. This is the cancellation-free scale. Relative to |F| alone, a target inside a sphere has a
nearly cancelling net force (|F|/gross as low as 1e-3), and the relative error there is mostly the oracle's own noise. The column
`total/|F|` is given for geometries with |F|/gross >= 0.05.

| tier | geometries | derived | pair (closed form vs 1-D integral) | total / gross | total / abs(F) (cond >= 0.05) | max leaves | cap hits |
|---|---|---|---|---|---|---|---|
| standard | 58 | 3.0e-15 | 9.3e-15 | 7.1e-03 | 8.6e-03 | 9.7e3 | 0 |
| validation | 58 | 3.0e-15 | 9.3e-15 | 1.6e-04 | 2.4e-04 | 3.6e6 | 0 |
| graded | 58 | 3.0e-15 | 9.3e-15 | 2.1e-05 | 1.4e-04 | 7.0e7 | 0 |
| certified | 10 (every 6th) | 2.8e-15 | 1.8e-15 | 2.9e-06 | 2.5e-05 | 1.6e8 | 0 |

Per class and tier, and the three worst geometries per tier: `report/data/ws6_xcheck_report.md`; every row: `report/data/ws6_xcheck_rows.csv`.
The engine refused none of the 58 geometries at any tier. The oracle's own self-estimate is at most 4.7e-5 (largest for class E).

What this says:

1. Derived quantities and the closed-form pair force agree with the independent evaluation to 1e-14, on all classes including target
   inside a sphere (class E) and near contact (class D, separation (a_0 + a_1)(1 + eps), eps from 1e-6).
2. The sparse class A has no overlapping shadows, so the total equals the pair sum to 2e-13 at every tier (the engine takes the pair path;
   leaves = 0).
3. For overlapping shadows the error is set by the quadrature tier, in the order standard > validation > graded > certified, in every class.
   The independent oracle reproduces the tier errors that WS3 measured on test (c) and the lens: standard about 1e-3 to 7e-3, validation
   about 1e-4, graded about 1e-5, certified about 1e-6.
4. On the 10 certified geometries the discrepancy (at most 2.9e-6) is within the oracle's own estimate for the same geometry in every case
   (e.g. geometry 48: 2.9e-6 against an oracle estimate of 2.7e-5). The certified tier is therefore verified to the oracle's resolution on
   those geometries, not to 1e-6. Two of the 10 certified geometries (12 and 18, class B) had no overlapping shadows and agree to 7e-14.
5. The 1e-6 target belongs to test (c) in the Spec. These results do not show that the graded tier meets 1e-6 on general geometries. On the 58
   random geometries it reaches 2.1e-5 of gross (6.5e-6 for class C deep overlap, 3.8e-6 for near contact). WS3_QUADRATURE.md states the same.
6. The `graded` tier was not worse than `validation` on any of the 41 geometries that have overlapping shadows (checked row by row in the CSV;
   the maxima above are about 8x smaller). It costs 16 to 42 times the leaves of `validation` on them (median 28).

Caveats: one IntegrationWarning (roundoff in a scipy `quad` call for the interior pair integral, epsabs 1e-15 is stricter than double precision
supports) was printed once in the run log (`report/data/ws6_xcheck_run.log`). The pair column for class E is 5.1e-15, so it had no effect. The first
run of this harness scaled the total by |F| and gave 2.8 for one class E geometry; that was the oracle's noise on a nearly cancelling force
(its own estimate was 2.4), not an engine error. The scale was changed to gross and the run repeated in full; the first run's report and rows
are kept in the evidence bundle (`first_run_scaled_by_abs_F_*`); the tables above are from the second run.

## 3. Property tests (`shadowgrav-core/tests/properties.rs`, std only)

Seeded xorshift64* generators, so every failure reproduces. Measured values are printed with `--nocapture`.

| test | what it checks | measured | bound |
|---|---|---|---|
| random_clusters_obey_permutation_translation_and_rotation_covariance | 12 random clusters (3 to 6 bodies, tau_c 0.5, 2, 5), standard tier | permutation 0, translation 1.0e-14, rotation about a random axis 3.9e-4 | 1e-9, 5e-3, 5e-3 |
| collinear_stacks_are_monotone_in_occluders_and_bounded_by_the_pair_sum | adding a body behind a collinear stack never lowers axial force; it never exceeds the sum of the pair forces (validation tier) | worst decrease 3.2e-5, worst excess 0 | 5e-4 |
| two_body_force_is_exactly_reciprocal_over_random_separations | 40 random separations and optical depths | 0 | 1e-12 |
| hostile_numbers_are_rejected_or_finite_never_a_panic | NaN, +-inf, 0, -1, 1e+-300, f64::MAX, MIN_POSITIVE, denormal through every global and particle field and the step | 99 constructions, 38 accepted with finite forces, 61 rejected, no panic | n/a |
| mutated_scenario_files_never_panic_the_parser_or_the_builder | 9000 byte-level mutations of the 6 shipped scenario files | 1099 still parse, 7901 rejected, no panic, finite forces whenever a world is built | n/a |

The monotonicity property follows from D growing pointwise when an occluder is added and cos(psi) >= 0 inside every disc of a collinear stack.
It is not true for non-collinear clusters, where the added shadow can reduce a transverse component. The two-body reciprocity test is exact by
construction (F_1 = -F_0 in the pair path); it is a regression guard, not independent evidence. The translation figure (1e-14) is much smaller
than the 1e-3 bound in `invariance.rs`, because the quadrature grid is anchored on the target, not on the axes.

The property tests were not themselves mutation-tested in this session (no deliberate bug was injected to see them fail).

## 4. Not done, and why

| item | reason | what to run |
|---|---|---|
| proptest | crates.io unreachable | add `proptest` as a dev-dependency and port the generators in `tests/properties.rs` |
| cargo-fuzz targets (scenario parser, `World::new`) | cannot install | targets would call `Scenario::from_json_text` and `World::new` on arbitrary bytes; seed corpus = `scenarios/*.json` |
| sanitizers (ASan/UBSan/MSan, Miri) | need nightly | `cargo +nightly miri test -p shadowgrav-core --test properties` (slow), `RUSTFLAGS=-Zsanitizer=address` |
| second implementation from the Spec PDF alone | PDF not available here | a reviewer who has not read this repository should implement eqs. 1-3 and compare with `xcheck_dump` |
| V1 / V2 / dynamics cross-check | out of this harness (V0 only) | extend `xcheck_random.py` with the velocity-dependent kernel |

The crate has no `unsafe` code. Both the release profile (`overflow-checks = true`) and the test profile (debug assertions on, overflow checks on, opt-level 3) trap
integer overflow. The whole offline core suite also passes in the test profile (22 suites, 133 passed, 0 failed, 4 ignored; `logs/test-profile.log`),
which is the available partial stand-in for sanitizers.
