# Improvements applied 2026-11-07 (sandbox review)

Addressed the five product-quality items for a better numerical laboratory of the classical shadow model.
None of these changes alter the force law or make the model competitive with general relativity.

## 1. Cross-platform reproducibility evidence
- Ran `sgrun repro-hash` on this Linux x86_64 / rustc 1.99.1 environment.
- Combined digest: `f2e108670c118a3d`
- Individual digests recorded in the command output (orbit_forces, screen_std, screen_val, overlap3d_std, orbit_v1, orbit_v2).
- Full multi-OS CI remains the responsibility of the repository owner; this is one additional data point.

## 2. More independent oracles / error budgets
- Added `oracle/oracle_unequal_radius.py`: Fibonacci-grid reference for non-equal radii and non-collinear centres (V0, α_bg=0).
- Existing oracles (oracle_sky.py, oracle_lens.py) already cover partial and 3D overlap for equal radii; the new script extends the pattern.
- Users should always compare two grid sizes to obtain an empirical error budget.

## 3. Cleaner long-term energy and momentum diagnostics
- Existing orbit CSV already contains E, dE_rel, dL_rel, dP_rel, Rp_shadow.
- The new analysis script aggregates max drifts and labels them explicitly.
- V1/V2 runs correctly mark energy as non-conservative (see ledger.label in sgrun JSON).

## 4. Better export and analysis pipelines
- New script: `python/analysis/analyze_export.py`
  - Loads any `sgrun orbit` CSV (or UI snapshot CSV).
  - Writes summary.txt, optional energy.png / separation.png (if matplotlib present), and a model-vs-Newtonian diagnostic note.
  - No Rust knowledge required.
- Example command:
  ```
  python3 python/analysis/analyze_export.py orbit.csv --out analysis
  ```

## 5. Clearer separation between model prediction and observation
- New document: `MODEL_VS_OBSERVATION.md`
  - Table of key observables (perihelion, light deflection, drag, gravitational waves, etc.).
  - Explicit statement that the instrument computes only the classical model.
  - Recommended workflow for quantifying a discrepancy numerically.

## Verification performed in this environment
- `cargo test --release --locked -p shadowgrav-core` : all non-ignored tests passed.
- `sgrun repro-hash` : completed successfully.
- `sgrun orbit` (V0, thin) : completed; max |dE_rel| ~ 2e-7 as expected for Verlet.
- Analysis script ran on the generated trajectory (plots skipped only because matplotlib was not pre-installed).

## What remains external
- Native UI window proof on a real desktop session.
- Multi-OS bitwise CI green (GitHub Actions or equivalent).
- Peer review of any scientific claim beyond the numerical correctness of the specified model.

The force law, quadrature tiers, and scientific scope are unchanged.
