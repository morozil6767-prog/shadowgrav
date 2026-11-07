# Usability pass (points 1 and 2 of the adoption plan)

All additions are outside the force path: `sgrun repro-hash` is unchanged (f2e108670c118a3d).

- `shadowgrav-core/src/scenario.rs`: std-only JSON reader + strict scenario format `shadowgrav-scenario/1` (unknown keys are errors).
- `sgrun run <scenario.json> [--out traj.csv]`, `sgrun validate <scenario.json>`: JSON result with provenance, per-body force/diagnostics, labels, optional reference check and tau_max.
- `shadowgrav-core/scenarios/`: three examples (thin two-body, test (c) collinear, 3D partial overlap).
- `shadowgrav-core/tests/scenario.rs`: 6 tests (parser, strictness, engine errors, Newton limit, R = 0.8).
- `python/`: dependency-free package that calls `sgrun` (no physics in Python); 5 unittest tests.
- `TUTORIAL.md`: format reference, three hand-checkable examples, reference-mode use, honest scope note on other attenuation problems.
- `.github/workflows/release.yml`: prebuilt zips on a version tag. NEVER RUN: check its first run.

Executed (sandbox S): core tests incl. scenario.rs, Python tests with a locally built sgrun.
Not executed: release workflow, Windows/macOS builds of the new code, UI changes (none made), Python on Windows.
Not done: Python bindings through PyO3 (needs crates.io), a `.sgs` binary state format, unit conversions for non-gravity use.
