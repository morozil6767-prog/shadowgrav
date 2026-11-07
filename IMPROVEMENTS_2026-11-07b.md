# Laboratory-quality improvements (2026-11-07, round 2)

Addressed the five remaining engineering items for a higher-quality numerical laboratory.
The force law is unchanged.

## 1. Native-window proof
- Enhanced `MACHINE_PROOF.md` with a clear report template and explicit guidance for headless/sandbox environments.
- Native visual proof still requires a real desktop session; this environment cannot produce screenshots of the egui window.
- Headless core tests continue to pass.

## 2. Multi-OS CI / reproducibility
- Created `REPRODUCIBILITY.md` with the current Linux x86_64 combined digest (`f2e108670c118a3d`) and a comparison table template.
- Full green multi-OS CI with attached logs remains an external step (requires macOS and Windows runners).

## 3. Error budgets for unequal-radius / non-collinear
- Created `ERROR_BUDGETS.md` with a concrete example geometry, tabulated N=2e6 vs N=8e6 results, and relative-difference estimates (~2e-4 on dominant components).
- The oracle script `oracle/oracle_unequal_radius.py` remains the regeneration path.

## 4. Analysis script wired into tutorial
- Appended §8 to `TUTORIAL.md` with the exact one-liner from `sgrun orbit` to plots.
- Improved `python/analysis/analyze_export.py` (better error handling, clearer messages, cross-reference to MODEL_VS_OBSERVATION.md).

## 5. Claims / observation documents kept in sync
- Added cross-reference section to `CLAIMS.md`.
- Added cross-reference section to `MODEL_VS_OBSERVATION.md`.
- Both now point at `ERROR_BUDGETS.md` and the analysis tools.

## Verification in this environment
- `cargo test --release --locked -p shadowgrav-core`: passed (previous run).
- `sgrun repro-hash`: combined `f2e108670c118a3d` (stable).
- Oracle script executed and results transcribed into ERROR_BUDGETS.md.

These changes raise the documentation and analysis quality of the instrument. They do not alter the classical shadow force law or its relationship to general relativity.

## Round 2b (continued)

- Added automatic `sgrun repro-hash` capture to the multi-OS CI workflow (`.github/workflows/ci.yml`). Digests will now appear in the uploaded artifacts on every Ubuntu/macOS/Windows run.
- Extended `ERROR_BUDGETS.md` with Geometry B (closer approach, mixed radii); relative grid difference ≤ 1.1×10⁻⁴.
- Updated `scripts/local-proof.sh` to also record the reproducibility digest.

These are incremental documentation and CI readiness improvements. Native-window screenshots and actual multi-OS runner logs remain external.
