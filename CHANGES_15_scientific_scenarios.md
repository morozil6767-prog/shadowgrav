# Engineering pass: automatic scientific summary + reference scenarios

Force path untouched.

- `sgrun run` JSON now includes a `scientific` object for recognised collinear three-body screening geometries:
  R, |R-0.8|, tier, and WITHIN/OUTSIDE status. Null for other geometries.
- New scenarios (all parse + build):
  - partial_overlap_planar.json
  - saturation_pair.json
  - drag_decay.json
- Python Result exposes `.scientific` and `.save_bodies_csv(path)`.

## Corrections after execution (Claude, sandbox; core 113 tests pass, Python 6 pass, repro-hash unchanged f2e108670c118a3d)
- `scientific` block: it compared R with 0.8 at ANY tau_c, so correct runs at tau_c = 1 or 5 (exact R 0.8778 / 0.8019) were reported "OUTSIDE", as were unequal-mass worlds. Now reported only for equal bodies with every tau_c >= 20; otherwise null.
- `drag_decay.json` aborted at step 1 (NonFiniteState): h = 1e-4 gives h*gamma ~ 5000 (gamma = 8.0e7). Now h = 6.25e-11 (h*gamma = 0.005); measured v/v0 = 0.36760 vs 1/e = 0.36788 after one decay time.
- New tests: every shipped scenario runs to completion; drag decay within 2e-3 of 1/e; Python test for the gating.
- Not verified: the `saturation_pair.json` title's "reduced by S^2" statement.
