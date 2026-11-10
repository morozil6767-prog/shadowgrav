# Changelog

Entries follow `CHANGES_*.md` in numeric order. `CHANGES_7`, `CHANGES_13`, and `CHANGES_18` are not in the tree. Later commits on `review/ws2c-ws6` are in `git log` and are not given a new summary here. No figure in this file was remeasured for the changelog.

## CHANGES_6

Test (c) xhi result `6.96e-7` did not reproduce; the file records xhi `3.32e-6`. `Validation::certified()` is added (`f_v` `7e-4`, cap 15). `sgrun` (`certify`, `scan`, `orbit`, `drag`, `skymap`, `repro-hash`), `converge`, and `build.rs` are added. The file says the full suite was not re-run and the egui files were uncompiled.

## CHANGES_8

UI borrow fix in `app.rs` (not compiled in that session). Trig calls routed through `mathx`. The file records default `sgrun repro-hash` `f2e108670c118a3d` before and after, and says the vendored-libm build was not compiled. Reference mode (`reference.rs`) for N ≤ 50, V0, exterior pairs. `edge_cases.rs` added. `ci.yml` added and marked never run.

## CHANGES_9

Written with no Rust toolchain in that session. Reference diagnostic and `tau_max` wired behind `SHADOWGRAV_EXTRA_DIAG=1`. Seeliger check and mid-run abort tests added and marked never run. The file says no traceability row became PASS.

## CHANGES_10

Quadrature tiers exposed in the UI: Standard, Validation, Fine, Certified. The file says the owner must recompile the UI. It keeps Standard at the provisional `5e-3` clause and says Validation and Fine are not Spec `1e-6`.

## CHANGES_11

Sky panel, force-component arrows, preset read-outs, CSV export (`export.rs`, key `E`), and a red status banner on a red badge or thin-fast. No new measurement is stated in the portion used here.

## CHANGES_12

Adds `CLAIMS.md`, `SCIENTIFIC_MANUAL.md`, `SCOPE_v1.md`, and the root README verification path. The file lists suites that passed in the authoring environment and leaves UI compile, multi-OS CI, the licence line, and the version tag as owner work.

## CHANGES_14

`scenario.rs` and `sgrun run` / `sgrun validate`. Three scenario files. `python/` calls `sgrun` and contains no physics. `TUTORIAL.md`. `release.yml` added and marked never run. The file says the release workflow and Windows/macOS builds were not executed.

## CHANGES_15

`sgrun run` gains a `scientific` object only for equal-body collinear screening with every `tau_c >= 20`. New scenarios `partial_overlap_planar.json`, `saturation_pair.json`, `drag_decay.json`. The file records a sandbox count of 113 core tests and 6 Python tests, and repro-hash still `f2e108670c118a3d`. It says the `saturation_pair.json` title's "reduced by S^2" line was not verified.

## CHANGES_16

`v2_valid` computed from velocities in every mode. `aggregates` and `spec_range_warnings` added to `sgrun` JSON. `invariance.rs` added. Force values unchanged; the file records repro-hash `f2e108670c118a3d`. UI display of the new fields was not wired.

## CHANGES_17

UI `rust-version` set to `1.88` after `1.85` was found false. `local-proof.sh` exit-status bug described. `git_state` values `clean` / `dirty` / `unknown`. Tutorial difference corrected to `7.5e-5` absolute and `7.5e-7` relative. The file's revision 3 says 129 Rust tests and 7 Python tests passed on rustc 1.88.0. That count is the file's count, not the later workspace total.

## CHANGES_19

Opt-in graded tier: `Validation.grade`, `Validation::graded()`, scenario `quadrature: "graded"`. Default `grade` stays `0`. The file says the default digest stays `f2e108670c118a3d` and that existing tests were not edited.

## CHANGES_20

`xcheck_dump` example, `oracle/xcheck_random.py`, and `tests/properties.rs` (five std-only tests). The file says no library source changed, the combined digest stayed `f2e108670c118a3d`, and the test count went from 128 to 133. It lists proptest, cargo-fuzz, and sanitizers as not done.
