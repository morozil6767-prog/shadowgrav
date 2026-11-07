# v1.0 scope statement

## In scope for v1.0

- Frozen mathematical specification + ADDENDUM2/3 (accepted scientific record)
- Conforming force core V0/V1; V2 available with amber caveats
- Certified / Validation / Fine / Standard quadrature tiers (UI + CLI)
- In-core reference mode (N ≤ 50, scoped)
- Automated tests: identities, integrator, oracles, reference, edge cases, diag gaps, perf equivalence (serial/parallel)
- Native egui instrument: presets, badge, sky map, force components, export CSV, thin-fast warnings
- Documentation: CLAIMS.md, SCIENTIFIC_MANUAL.md, README quickstart, TRACEABILITY

## Explicitly deferred (not v1.0 blockers)

| Item | Reason |
|------|--------|
| Full cross-OS bitwise CI green | Requires GitHub (or equivalent) multi-runner results owned by the releaser |
| GPU / WASM builds | Not required for desktop research use |
| Scene editor / arbitrary IC file format beyond presets + export | Export is enough for paper workflows; editor is v1.1+ |
| Certified-tier interactive long Play | Cost ~10–20 s/eval; Step/single-eval is the supported certified workflow |
| Exhaustive many-body geometry atlas | Partial/3D oracle cases + collinear test (c) define the verification spine |
| Publication peer review | Process, not code |

## Residual risks acknowledged in v1.0

- Rim floor on certified test (c) at a few ×10⁻⁷  
- V2 less exercised than V0/V1  
- `skymap` diagnostic path may still use some platform trig (forces path routed through mathx)  
- UI must be compiled on the owner’s toolchain (egui/eframe)

## Exit criterion for tagging v1.0

1. `cargo test --release -p shadowgrav-core` green on owner machine  
2. `cargo run --release -p shadowgrav-ui` launches; presets (a)(b)(c) behave; export works  
3. CLAIMS.md matches measured numbers  
4. README “10 minutes” path works for a second person (or is recorded as self-checked)
