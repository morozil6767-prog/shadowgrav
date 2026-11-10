# WS3: why the cheaper tiers miss 1e-6 on test (c), and an opt-in graded tier

Evidence tags: [RAN] executed, [READ] inspected, [INFERRED] reasoning. All [RAN] numbers come from the serial core (no `rayon`, no `libm` crate) on rustc 1.97.0, 2 cores, one thread, and are
reproducible from `report/data/ws3_graded.csv` (raw logs: the evidence bundle of the WS3 pass). Timings were taken on one machine and are indicative only.
**Scope:** only silhouette-aware refinement was investigated. Exact handling of fully nested discs and higher-order leaf rules were **not** investigated. The Spec, its addenda and every existing tolerance are unchanged.

## 1. What limits the existing tiers [RAN]

Test (c), tau = 20, reference R_exact = 0.800000002009972 (oracle, 30 digits). In the existing rule a cell is refined only if it is too coarse for the interior (rho > f_v theta_min) or touches a silhouette
(|psi - theta| <= rho and rho > f_s theta_eff). A cell that is merely *near* a silhouette is left at the interior level. At the silhouette the integrand has a square-root kink (the chord length goes to zero), so
those coarse neighbours carry a midpoint error of order h^(3/2) each.

Two measurements point at those neighbours and not at the rim cap or the smooth interior:

* Changing the rim cap from 15 to 16 (f_s 4e-4 to 1e-9) shifts R by the same -2.68e-7 in every one of seven interior settings (f_v = 0.08 ... 0.00125), while the error itself changes by a factor of 15 over the same settings.
* At the coarsest interior (f_v = 0.08), adding refinement of non-touching cells near the silhouette (see section 2) cuts the error from 2.05e-5 to 4.87e-6 (kappa = 1) and 2.05e-6 (kappa = 0.5) at the same interior level.

The error of the ungraded tiers also falls only like leaves^-0.7 to -0.9 (`CONVERGENCE.md`), consistent with an h^(3/2) term dominating.
[INFERRED] A residual that depends on kappa but not on the rim cap (kappa 0.5: -2.26e-6, kappa 0.25: -0.82e-6, both unchanged from cap 16 to cap 18 and within 2.5e-7 of their cap-15 values; ratio 2.75, exponent about 1.5 in kappa) is most likely the midpoint error of the graded zone itself. That is a hypothesis; it was not tested.

## 2. The rule (`Validation::grade`, kappa; 0 = off) [READ] `src/overlap.rs`

For a cell that does not touch a silhouette (gr > rho, gr = distance to the nearest disc boundary among the touching discs), refine it if `rho > kappa (gr - rho)`. Cell size then falls geometrically toward each rim.
Refinement requested only by this rule is never counted as a cap hit. With `grade = 0` the code path is unchanged: digest `f2e108670c118a3d`, all 125 earlier tests and 4 ignored tests still pass [RAN].

Preset `Validation::graded()` = { f_v 0.0025, f_s 4e-4, ell_cap 15, grade 0.125 }; selectable as `quadrature: "graded"` in scenario files and `--mode graded` in `sgrun`. The UI does not expose it.

## 3. Results [RAN]

Test (c) against the exact curve (|R - R_exact|; leaves; idle single-thread seconds at tau = 20):

| Tier | tau = 1 | tau = 5 | tau = 20 | leaves | seconds |
|---|---|---|---|---|---|
| certified (7e-4, 4e-4, 15, 0) | 4.74e-8 | 1.82e-7 | 4.31e-7 | 64,767,596 | 16.8 |
| **graded** (0.0025, 4e-4, 15, 0.125) | 1.51e-7 | 3.21e-7 | 1.50e-8 | 9,884,156 | 3.5 |
| xhi (ungraded, 0.002) | 5.56e-7 | 1.95e-6 | 3.32e-6 | 4,467,284 | 1.1 |

Worst case over the three tau: 3.2e-7 (graded) against 4.3e-7 (certified); 6.6x fewer leaves, 4.8x less time. No cap hits.
`report/figures/ws3_convergence.png` shows the whole picture. **The advantage is not uniform:** at tau = 1 the graded ladder crosses the ungraded curve only above about 7M leaves, at tau = 5 it is about 4.8x better at 15M leaves, and at tau = 20 it is far better.

**Sensitivity of tau = 20.** The nominal tau = 20 error (1.5e-8) is smaller than the tier's own sensitivity to the rim cap: with the rim refined (f_s 1e-4, cap 16, 17.1M leaves) the error is -1.92e-7, and tau = 5 becomes -3.99e-7. The defensible statement is therefore "largest of nominal and rim-refined error = 4.0e-7 (2.5x under 1e-6)", not "1.5e-8".

**Ladder** (kappa and f_v halved together, cap 15): leaves 3.57M, 7.06M, 15.3M, 36.6M; tau = 1 signed errors -1.66e-6, -4.29e-7, -9.68e-8, -1.04e-8; tau = 5: -2.88e-6, -7.45e-7, -1.41e-7, +1.93e-8; tau = 20: -5.46e-7, -1.86e-8, +1.42e-7, +1.84e-7.
For tau = 1 and 5 each rung reduces the error by 3.9x to 9x (3.9x, 4.4x, 9.3x at tau = 1). At tau = 20 the ladder flattens at +1.8e-7 because the rim (cap 15) then dominates, and Richardson extrapolation with ratio 4 does not apply there (it gives +1.6e-7 from the first pair, worse than the unextrapolated rung).
For tau = 1 and 5, extrapolating pairs of rungs with ratio 4 gives errors of 1.9e-8 / 1.4e-8 (tau = 1) and 3.5e-8 / 6.0e-8 (tau = 5).

**Self-convergence indicator** (usable when no oracle exists): |R(tier) - R(next rung)| with the next rung = (f_v 0.00125, kappa 0.0625) is 1.27e-7 against a true tier error of 1.51e-7 (tau = 1), 2.94e-7 against 3.21e-7 (tau = 5) and 1.37e-7 against 1.5e-8 (tau = 20). It is a rough guide that under-read by 9 to 16 percent at tau = 1 and 5 and over-read at tau = 20, **not a bound**.

**Partial and 3D overlap against the sharp lens oracle** (max |dF|/|F|; leaves), `tests/oracle_lens.rs` cases:

| Case | validation | certified | graded |
|---|---|---|---|
| x2 = (12, 2, 0), tau 1 | 5.69e-6; 61,196 | 2.73e-8; 961,208 | 6.80e-8; 1,433,714 |
| x2 = (12, 2, 0), tau 5 | 2.01e-5 | 1.48e-7 | 2.05e-7 |
| x2 = (12, 1.4, 1.4), tau 1 | 5.43e-6; 64,190 | 2.63e-8; 1,096,775 | 6.65e-8; 1,507,553 |
| x2 = (12, 1.4, 1.4), tau 5 | 1.87e-5 | 1.40e-7 | 1.98e-7 |

On these cases the graded tier is 80x to 100x closer to the oracle than validation mode, has 1.4x to 2.5x the error of certified, and uses 1.4x to 1.5x **more** leaves than certified. Its cost advantage is specific to the nested-disc geometry of test (c).

## 4. Limits

* Accuracy is established only against the oracles above: the exact test (c) curve at tau = 1, 5, 20 and four lens/3D configurations. There is no error bound, and a different geometry may behave differently. The sky oracle (noise about 6e-5) cannot resolve this level.
* Serial, default libm, one machine. The parallel and vendored-libm paths and the UI were not run.
* Adopting `graded` as the Spec's certified tier is a Spec decision for the owner; nothing in the Spec text was changed.
* Open question from section 1: whether a higher-order leaf rule near the rim, or exact nested-disc handling, removes the kappa-dependent floor at lower cost.

## 5. Reproduce

```
cargo test --release -p shadowgrav-core --test oracle_sky -- screening_curve_graded grading_adds --nocapture
cargo test --release -p shadowgrav-core --test oracle_lens -- --include-ignored --nocapture
python3 report/make_ws3_figure.py          # needs matplotlib
```
