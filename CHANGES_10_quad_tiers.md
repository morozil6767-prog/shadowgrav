# workspace_10-style pass: first-class quadrature tiers

## Product change
Certified is no longer CLI-only. The UI exposes an explicit **Quad tier** control:

| Tier | Core mapping | Test (c) expectation |
|------|----------------|----------------------|
| Standard | validation=None (f=0.15) | |R-0.8| <= 5e-3 provisional |
| Validation | Validation::default_validation (f_v=0.02) | ~1e-5 |
| Fine | Validation::xhi (f_v=0.002) | ~3.3e-6 |
| Certified | Validation::certified (f_v=7e-4) | <= 1e-6 (measured ~4e-7) |

## Files
- shadowgrav-ui/src/protocol.rs — `QuadTier`, `Command::SetQuadTier`, `ParamView.quad_tier`
- shadowgrav-ui/src/sim.rs — tier mapping, screening/partial regime status
- shadowgrav-ui/src/app.rs — combo box + certified cost warning

## Honesty retained
- Standard remains provisional 5e-3
- Validation/Fine are not Spec 1e-6
- Certified meets Spec 1e-6 on the test (c) geometry and warns it is slow

Owner must recompile the UI.
