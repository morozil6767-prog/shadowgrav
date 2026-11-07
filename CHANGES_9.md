# workspace_9: changes since workspace_8

**Nothing here was compiled or run.** This session had no Rust toolchain and no network. Everything below is written and read-checked only
(brace balance checked mechanically; the Seeliger bound was reproduced in Python). Expect ordinary compile slips; no Traceability row became PASS.

## Code
1. **Reference error wired into diagnostics (Spec 6 item 11).** `diagnostics::reference_diag` (wraps `reference_check`; out-of-scope worlds give "not checked: <reason>", never a pass),
   `Snapshot.reference` via opt-in `snapshot_with(w, &SnapshotOptions{..})` (`snapshot()` unchanged). UI: `DiagView.ref_check` + `tau_add_max` (protocol.rs), filled in `sim.rs` only when
   env `SHADOWGRAV_EXTRA_DIAG=1`, and shown as two "Preset read-outs" rows through the existing `extras` list. No egui widget, no layout change. Live view skips the reference for N > 8.
2. **tau_max (Spec 6 item 6).** `diagnostics::tau_max`: lower bound of the additive optical depth over {directions to other bodies' centres} and, when instrumented, {leaf centres}.
   **Definition reconstructed from ADDENDUM3 item 8 (Spec text not in the workspace): please confirm** (ADDENDUM3 item 11 lists the four assumptions).
3. **Paired Seeliger (Spec 6 item 8).** `diagnostics::seeliger_check(w, alpha)`: F(alpha)/F(0) per pair and per body (two full force evaluations, `w` untouched), against exp(-alpha d). V2 and bad alpha refused.
4. **A7 mid-run abort.** `tests/diag_gaps.rs`: NaN velocity, drift overflow (`NonFiniteState`) and non-finite force (`Core(NonFinite)`, Pi = Inf), each with bitwise state restoration; plus the same force through a live `apply_globals` edit.

## Tests: `tests/diag_gaps.rs`, 11 tests, never run
`cargo test --release -p shadowgrav-core --test diag_gaps`. Every bound is derived (stated beside it), none measured. [P] Seeliger deviation for the test geometry: 1.0e-6 against the 2e-3 bound.

## Documents
TRACEABILITY.md: A7, E7, E9, E11 rows updated; totals **39 PASS / 18 PARTIAL / 3 GAP** (recounted by script; E7 GAP -> PARTIAL; nothing new is PASS because nothing was executed).
ADDENDUM3.md: items 8 (table) and new item 11. README.md, VERIFY.md (Stage 6) updated. Performance claims and UI layout untouched.

## Files touched
shadowgrav-core/src/diagnostics.rs, tests/diag_gaps.rs (new); shadowgrav-ui/src/protocol.rs, sim.rs (uncompiled); TRACEABILITY.md, ADDENDUM3.md, README.md, VERIFY.md, README_WORKSPACE.md, CHANGES_9.md.

## Tomorrow, in order
1. `cargo build --release`, then `cargo test --release -p shadowgrav-core --test diag_gaps`; report compiler output / failures verbatim.
2. Recompile the UI (workspace_8's E0503 fix is also still uncompiled, and sim.rs/protocol.rs changed).
3. Push, read CI, compare the repro-hash digests (unchanged by this round: no force-path code was touched).
