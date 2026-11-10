# CHANGES 17: fixes for the 2026-11-07 baseline audit findings

**Status: first revision verified by an independent run (129 Rust + 7 Python tests passed on rustc 1.99.1); that run found three defects in
this change, corrected in the "Revision 2" section below. Revision 2's Rust edits were written without a toolchain (bracket-balance and
read-checked only), so by the ADDENDUM3.md evidence legend they are **N** until `cargo test --release --workspace` and a launch are run on
rustc >= 1.88 (the audits used 1.99.1).**

| Finding | Change | Checked |
|---|---|---|
| `rust-version = "1.76"` in `shadowgrav-ui` is false | Revision 1 said 1.85, which was **wrong**; see Revision 2 | Core still says 1.70, which has never been run below 1.83 (geometry_suite and identities pass on 1.85.0); left unchanged for lack of evidence |
| TUTORIAL says forces differ by 7.5e-6 | Now "7.5e-5 absolute, 7.5e-7 relative" (the run's `delta_sat`) | Arithmetic from the audit values |
| `local-proof.sh` stored the `repro-hash` exit status as `ui_build_exit` | UI build status captured immediately after the build; `repro-hash` has its own `repro_hash_exit`; final exit status includes both | `bash -n` only; not run end to end |
| `stair_model.py` fails outside `oracle/` | Path resolved relative to the script | Run from repo root and from `oracle/`: same output |
| `build.rs` calls a clean tree "clean-or-unknown" | `git_state` is now `clean`, `dirty` or `unknown` (git missing / not a repo) | Not compiled |
| Arrow legend overprinted the keys hint (top right of the 3D view) | Hint is fitted to the free width beside the legend and conformance stamp, falling back to shorter text; brighter grey | Not compiled or viewed |
| Scale-bar label ran past the right edge | Label right-aligned at the bar's right end | Not compiled or viewed |
| "engine vs all-rays reference 3.08e-5" read as an engine error | Row renamed "all-rays cross-check (reference error limits it)"; `ReferenceDiag::summary` states the gap, the reference's own quadrature error and whether the gap is within it; for two disjoint bodies the UI adds that the engine force is the exact closed form | `diag_gaps` only asserts the `not checked` prefix and `V0 only`; both preserved |
| `geometry_suite` layouts test used absolute units (every pair >= 280 a apart, no overlap) | Positions now in units of `a`; layouts 1 and 2 must report `overlap_present` | Overlap predicted by an independent Python calculation (separation 0.9 x summed angular radii); engine behaviour not run |

Not changed: test (a) tolerance (needs a decision, see the review notes), the tier-dependent test (c) claims, UI warnings, CI.

## Revision 2 (after independent verification)

| Verification finding | Change |
|---|---|
| rustc 1.85 cannot build the locked UI graph: `calloop` 0.14.5 needs 1.86, `icu_*` 2.4.0 and `image` 0.25.10 need 1.88 | UI `rust-version` is now `1.88`; both READMEs name the three crates. 1.88 is the highest floor cargo reported, so it is necessary; only 1.99.1 has actually built the UI |
| Default startup lost the 3D view: the cross-check sentence (~180 characters, unwrapped) widened the right-hand panel | The live read-out now shows short rows (max rel diff, reference's own error, within/exceeds, a short pair note). The full sentence stays in the export and `sgrun` JSON. The right panel also gets `max_width(560.0)` as a safety net |
| At 960x700 the colour label ran through the arrow legend | The colour label uses the same fit-to-free-width rule as the hint, with shorter fallbacks |
| `git_state` stays stale until `build.rs` is rebuilt | `build.rs` now also reruns on `.git/index` changes. Adding a purely untracked file still does not re-stamp (cargo has no hook for it); stated in a comment |
| Disjoint-body note not seen | It now appears as its own short row when the gap is within the reference error |

Still open: test (a) (1.34e-5 against the PDF's 5e-6; needs a policy decision), 20 UI warnings including the `f32` fallback that will become a hard error, no macOS/Windows/CI run.

## Revision 3

Verification of revision 2 (129 Rust tests passed on rustc 1.88.0, 7 Python tests passed, repro hash unchanged) found one script defect: `git status`
refreshes `.git/index`, which `build.rs` watches, so builds re-ran the script needlessly and the "untracked file" comment was wrong.
`build.rs` now runs `git status` with `GIT_OPTIONAL_LOCKS=0` (reproduced separately in a scratch repo: plain status rewrites the index when file
stamps are stale, this setting does not), and the comment states exactly what does and does not re-stamp. Not compiled in this session.
