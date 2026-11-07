# Audit follow-ups (2026-11-07)

An independent reviewer compared the core with the Spec (code read only). No formula, sign or factor contradicted the Spec.
Full register: `shadowgrav-core/ADDENDUM4.md` (all items PROPOSED until the owner signs).

Code (force values unchanged; `sgrun repro-hash` f2e108670c118a3d):
- `force.rs`: `v2_valid` is now computed from the velocities in every mode (was always true outside V2).
- `diagnostics.rs`: `aggregates` (tau_c, S, D_bar min/mean/max, total heating power; None = not measured) and `spec_range_warnings`.
- `sgrun run` JSON: new `warnings` and `aggregates`; Python `Result.warnings`, `.aggregates`.
Tests: 4 new in `diag_gaps.rs`; new `tests/invariance.rs` (permutation 0, translation 2.6e-15, rotation 1.3e-4 standard / 1.0e-6 validation, 1/r^2 law, dense N=12 momentum residual 6.3e-4).
Docs: CLAIMS.md version line no longer says "UI instrument-grade; packaging complete".
Executed: sandbox only. Not wired: UI display of aggregates/warnings.
