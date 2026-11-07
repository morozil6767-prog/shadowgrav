# Error budgets for non-collinear and unequal-radius geometries

This document records independent Fibonacci-grid oracle results for geometries beyond the equal-radius collinear and partial-overlap cases already covered by `oracle_sky.py` and `oracle_lens.py`.

All results are for V0, α_bg = 0. The oracle uses equal solid-angle Fibonacci sampling. Accuracy is estimated by comparing two grid sizes; the larger grid is reported. Rim discontinuities limit absolute accuracy to roughly 10⁻⁴–10⁻⁵ relative for these N values.

## Geometry A — unequal radii, non-collinear (example)

Centres: `[[0,0,0], [12,0,0], [6,8,1]]`  
Radii: `[1.0, 1.0, 1.5]`  
τ_c: `[5.0, 5.0, 8.0]`

| Target | N = 2×10⁶ (F/Γ) | N = 8×10⁶ (F/Γ) | Max component rel. diff |
|--------|------------------|------------------|-------------------------|
| 0 | [6.0639e-2, 5.3983e-2, 6.747e-3] | [6.0626e-2, 5.3979e-2, 6.748e-3] | ~2×10⁻⁴ |
| 1 | [-6.0618e-2, 5.3986e-2, 6.748e-3] | [-6.0623e-2, 5.3977e-2, 6.747e-3] | ~2×10⁻⁴ |
| 2 | [5.9e-6, -4.5722e-2, -5.715e-3] | [-1.0e-6, -4.5715e-2, -5.714e-3] | ~1×10⁻³ (small component) |

**Recommended use:** Treat the N = 8×10⁶ result as a reference with an empirical uncertainty of a few ×10⁻⁴ relative on the dominant components. Production quadrature at Validation tier should be compared against this reference; larger residuals indicate either geometry-specific quadrature difficulty or a bug.

## How to regenerate

```bash
python3 oracle/oracle_unequal_radius.py
```

Extend the script with additional centres/radii/τ_c lists and re-run to publish further budgets. Always report both N values.

## Relation to production path

These oracles use an independent sampling method (Fibonacci grid, no HEALPix, no pair closed form). Agreement within the grid uncertainty supports the production adaptive quadrature for the tested geometries. Disagreement larger than the grid uncertainty should be investigated before quoting forces.

## Geometry B — closer approach, mixed radii

Centres: `[[0,0,0], [6,0,0], [3,4,0.5]]`  
Radii: `[1.0, 1.2, 0.8]`  
τ_c: `[10.0, 4.0, 6.0]`

| Target | N = 2×10⁶ (F/Γ) | N = 8×10⁶ (F/Γ) | Rel. diff |
|--------|------------------|------------------|-----------|
| 0 | [1.5636e-1, 5.9935e-2, 7.491e-3] | [1.5634e-1, 5.9928e-2, 7.491e-3] | 1.1×10⁻⁴ |
| 1 | [-1.3046e-1, 5.9931e-2, 7.490e-3] | [-1.3047e-1, 5.9925e-2, 7.491e-3] | 7.0×10⁻⁵ |
| 2 | [2.202e-2, -2.2349e-1, -2.794e-2] | [2.202e-2, -2.2348e-1, -2.794e-2] | 1.5×10⁻⁵ |

Dominant-component relative difference between the two grids is ≤ 1.1×10⁻⁴. Use the N = 8×10⁶ column as the reference with that empirical uncertainty.
