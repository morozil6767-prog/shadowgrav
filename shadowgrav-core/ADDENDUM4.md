# ADDENDUM 4: deviation register from the independent audit of 2026-11-07

Source: a fresh reviewer (no prior context) compared `shadowgrav-core/src` with the frozen Spec text. Result: **no formula, sign or
factor contradicts the Spec**; the items below are places where the code differs in procedure or reporting from the Spec wording and no earlier
Addendum records it. They are listed so the owner can accept, amend the Spec, or change the code. **Status of every item: PROPOSED, not accepted.**
Each "executed" statement was run in the sandbox (S); nothing here has been run on another platform.

| # | Spec clause | Code | Effect | Proposed disposition |
|---|---|---|---|---|
| 1 | 6.12 V2-valid flag | was true in V0/V1 whatever the speed | wrong flag | **FIXED in code** (`force.rs`); test `v2_valid_flag_reports_fast_bodies_in_every_mode` |
| 2 | 1.2 "tau < 0.05 evaluate the series" | `derive.rs` SERIES_TAU = 1.0 (40 terms) | identical function; coefficients checked | amend Spec to tau < 1 |
| 3 | 2.3 interior P panels | extra kink breakpoints and dyadic grading (`pair.rs`) | same integral; ADDENDUM2 item 2 covers the rewrite, not the extra panels | add to ADDENDUM2 |
| 4 | 2.3 exterior GL in phi | graded sub-panels when 2a(alpha+beta) > 4 | same integral, more nodes | add to ADDENDUM2 |
| 5 | 6 "diagnostics use the same quadrature as the force" | Omega, tau_bar, E use a sinh substitution | same integrals, different nodes | amend Spec wording |
| 6 | 4.2 D = -expm1(-sum tau) | recursion X <- X + D_j(1-X) | algebraically identical, avoids cancellation | amend Spec |
| 7 | 3.4, 6.1 use G_N | `diagnostics.rs` uses xi Pi kappa0^2 / 4 pi | equal when calibrated (eq. 19); differs only in uncalibrated mode | amend Spec or use G_N |
| 8 | 3.4 E valid for V0, alpha_bg = 0, non-overlapping or equal radius | label also requires no sky-disc overlap | stricter (safe) | amend Spec |
| 9 | 4.3 item 6 "D_k(n_c) - sum D_kj" | D_k evaluated over the touching set | physically required; reading of an ambiguous formula | amend Spec |
| 10 | V2 pair part | semi-analytic eq. 9 with lambda^2/w Jacobian and eq. 4 decomposition; reduces to eq. 10 at first order (checked) | Spec ambiguity | record as interpretation |
| 11 | 4.4 / 6.11 reference mode | scope limited to V0, alpha_bg = 0, bodies, exterior pairs, N <= 50 | returns "not checked", never agreement | document (done in CLAIMS) |
| 12 | 6 item 4 total power; item 6 tau_c, S min/mean/max; item 7 D_bar mean/max | were missing in core | missing diagnostics | **ADDED** (`diagnostics::aggregates`), tests added; UI display not wired |
| 13 | 6 parameter table ranges | not enforced in core (wider domain for verification presets) | silent out-of-table values | **ADDED** `spec_range_warnings`, shown in `sgrun run` and Python `.warnings`; core still accepts them |
| 14 | 3.4 conservation "every step" | `diag_every` default 1, configurable | matches by default | document |
| 15 | 6 item 10 drift denominator | uses the current sum m|v| | Spec does not say initial or current | owner to choose |
| 16 | 6 item 1 probe G_eff(r) | r = m^g-weighted centroid | Spec does not define r | owner to choose |

Not covered by this audit: the UI crate, the build system, anything requiring a run (the reviewer read code only).
