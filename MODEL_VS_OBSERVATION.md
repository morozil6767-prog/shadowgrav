# Model predictions vs. observation

This instrument computes the force law of one classical, non-relativistic shadow-gravity (Le Sage-type) model.  
It does **not** compute general relativity. The table below lists key observational constraints and the qualitative status of the classical model.  
Use the instrument to quantify the numerical size of the discrepancy for any concrete geometry you choose; do not treat the numbers as predictions for the real universe.

| Observable | GR / measurement | Classical shadow model (this instrument) | How to quantify with shadowgrav |
|------------|------------------|------------------------------------------|---------------------------------|
| Inverse-square static force (weak field, well-separated) | Matches Newton to high precision | Recovers Newtonian limit for thin spheres (S → 1) | Two-body orbit, thin τ; compare period and energy to Newtonian |
| Perihelion precession of Mercury | +42.98″/century beyond Newton | No GR-like precession; pure Newtonian (or small finite-size corrections) | Long orbit integration; measure apsidal advance |
| Light deflection by the Sun | 1.75″ | Zero (model is not about electromagnetism or null geodesics) | Not applicable |
| Shapiro delay / radar ranging | Matches GR | Not present | Not applicable |
| Gravitational redshift / clock rates | Matches GR | Absent (classical non-relativistic) | Not applicable |
| Velocity-dependent drag on Solar-System bodies | None observed at the level required by a static isotropic flux | Present (V1 term); magnitude set by the flux parameters | Run `sgrun orbit --mode v1`; measure decay of orbital energy / eccentricity damping |
| Gravitational waves | Detected; propagate at c | Not present | Not applicable |
| Binary-pulsar timing (orbital decay) | Matches GR quadrupole formula | Drag + no GW emission | Compare energy-loss rate to the GR prediction for the same system |
| Frame-dragging / Lense-Thirring | Measured (Gravity Probe B, LARES) | Absent | Not applicable |
| Strong-field black-hole shadows | Matches GR | No event horizons | Not applicable |

## Recommended workflow for a discrepancy study
1. Choose a simple system (two-body orbit, three-body screening, etc.).
2. Run the instrument at Validation or Fine tier.
3. Export the trajectory or force snapshot.
4. Use `python/analyze_export.py` (or your own script) to extract the residual relative to the Newtonian expectation.
5. Compare the residual magnitude to the observational upper bound on any anomalous force or drag.

The instrument is a numerical laboratory for the model.  
The observational column is external to the code; the separation is intentional.

## Cross-references

- Numerical error budgets for geometries beyond the certified collinear test: see `ERROR_BUDGETS.md`.
- Claims the instrument is permitted to make: see `CLAIMS.md`.
- How to quantify a residual with the analysis tools: see `TUTORIAL.md` §8 and `python/analysis/analyze_export.py`.

Any number quoted from a run should cite the quadrature tier, geometry, and (when available) the corresponding oracle error budget.
