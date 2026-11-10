# Tutorial: three checks you can verify by hand, then your own problem

Everything here uses `sgrun` (built from this repository) and, optionally, the small Python package in `python/`.
All numbers below were produced by the engine in this repository (Linux build, sandbox run; re-run them on your machine).

## 0. Build or download

```
cargo build --release --locked -p shadowgrav-core --bin sgrun      # produces target/release/sgrun
```
or unzip a prebuilt `shadowgrav-<platform>.zip` (see the release workflow; prebuilt files exist only after the first tagged release).

## 1. Scenario files

A scenario is a plain JSON file. **Unknown keys are errors** (a typo never silently becomes a default).

See `shadowgrav-core/src/scenario.rs` and the examples in `shadowgrav-core/scenarios/` for complete files. Keys:

| key | meaning |
|---|---|
| `format` | required: `"shadowgrav-scenario/1"` |
| `mode` | `v0` (default), `v1`, `v2` (v2 is caveat-flagged: less validated) |
| `quadrature` | `standard` (about 1e-3 relative), `validation` (about 1e-5), `xhi`, `certified` (slow, about 4e-7 on test (c)) |
| `length_unit` | `a0` (default; radius of the m=1 reference sphere) or `code` |
| `globals` | exactly one of `tau_c` (optical depth of the reference sphere) or `kappa0`; optional `g_n`, `xi`, `v_g`, `alpha_bg`, `w:[x,y,z]`, `pi` (uncalibrated mode) |
| `bodies` | list of `{ "x":[..], "v":[..], "m":1, "rho":2.39e5 }`; only `x` is required |
| `step` | `{ "h": 1e-4 }` or `{ "adaptive": { "h_max": .., "eta_t": 0.005 } }` (adaptive runs are flagged non-reversible) |
| `run` | `{ "steps": N, "stride": k }`; `steps: 0` evaluates forces at t=0 only |
| `leaf_budget` | 0 = unlimited; exceeding it aborts instead of silently coarsening |
| `diagnostics` | `{ "reference": true }` compares against the independent all-rays reference (N<=50, v0, exterior pairs); `{ "tau_max": true }` |

Run it:

```
sgrun validate scenario.json                       # parse and build only
sgrun run scenario.json --out trajectory.csv       # JSON result on stdout (with provenance), CSV of sampled steps
```

Every result carries `provenance` (engine version, git commit, compiler, OS, thread count). Quote it with any number you publish.
Also read `labels` and `status` in the result: they say when a run is outside the validated regime (cap hit, adaptive step, non-conservative energy...).

## 2. Check A, two bodies in the thin limit: Newton's law

`shadowgrav-core/scenarios/two_body_thin.json`: tau_c = 1e-6, bodies 10 a0 apart, a0 = (3/(4 pi 2.39e5))^(1/3) = 0.01 (the m=1 sphere radius).
Hand calculation: F = G m^2 / r^2 with G = m = 1, r = 10 a0, so F = 1/(10 a0)^2.
Engine: F = 100.07463; hand: 100.07471. They differ by 7.5e-5 absolute, 7.5e-7 relative, which is the thin-limit saturation error of tau_c = 1e-6 (the run's delta_sat).

## 3. Check B, the screening ratio of three collinear spheres

`shadowgrav-core/scenarios/collinear_screening.json`: spheres A, B, C on a line, spacing 10 a0, tau_c = 20 (very opaque).
At tau_c = 20, B hides C completely from A (equal radii, collinear, saturated), so A feels only B. The pair-additive force is
F_AB + F_AC = 1/100 + 1/400 = 1.25/100 (units of 1/a0^2), the true force is 1/100, so the ratio is **R = 1/1.25 = 0.8** up to
corrections of order exp(-tau): the exact value is R = 0.8 + 2.0e-9.
Engine, `quadrature: "validation"`: R = 0.799984 (error 1.6e-5, as documented for that tier). `certified` reaches about 4e-7 and takes about 14 s.

## 4. Check C, R as a function of opacity (and how to see saturation)

```python
import shadowgrav as sg
for t in (0.01, 0.1, 1, 5, 20):
    r = sg.run(bodies=[[0,0,0],[10,0,0],[20,0,0]], tau_c=t, quadrature="validation")
    print(t, round(r.screening_ratio(), 6))
```
Output: 0.01 -> 0.998113, 0.1 -> 0.981916, 1 -> 0.877773, 5 -> 0.801839, 20 -> 0.799984.
Independent exact values (Python oracle in `oracle/`): R(1) = 0.877785, R(5) = 0.801868, R(20) = 0.800000002.
The engine agrees to 1e-5 to 3e-5 at this quadrature tier. R falls from 1 (additive) to 0.8 (fully screened), which is the non-additivity the instrument measures.

## 5. Use the reference mode to check your own geometry

Add `"diagnostics": {"reference": true}`. The result's `reference.summary` says how the production engine compares with an independent
all-rays integration of the same shadow model, and whether the difference exceeds the reference's own resolution estimate.
Example (`partial_overlap_3d.json`): max difference 2.8e-5 against a reference quadrature error of about 1.7e-5, so "the gap exceeds it": a reminder that this geometry is only good to the stated tier. If a world is outside the reference's scope the result says "not checked", never "agrees".

## 6. Python

`pip install -e python` (no dependencies; NumPy is used if present). `python -m unittest discover -s python/tests` runs the checks above against your `sgrun`.
The package contains no physics: it writes the scenario file, calls `sgrun`, and reads the JSON/CSV. Set `SHADOWGRAV_SGRUN=/path/to/sgrun` if it is not on PATH or in `target/release`.

## 7. What this tool is, and is not, for

Engine scope: uniform finite spheres attenuating an isotropic momentum flux; net force F_k = Gamma_k * integral n D_k(n) dOmega with D = 1 - exp(-sum beta_j L_j). That is the shadow-gravity model in `SCOPE_v1.md`.

**Other attenuation problems (untested idea).** The same mathematics describes any isotropic flux shadowed by opaque-ish spheres (radiation pressure among grains, self-shielding of particle clusters). The mapping would be: `tau_c` is the optical depth of a reference sphere, the masses and densities set sphere radii, and `G_N`/`xi`/`pi` set the force scale. This mapping has not been validated against any other physical system, no unit conversion is provided, and V1/V2 are specific to the gravity model. Treat any such use as exploratory until you have checked a case against an independent calculation of your own field, using the reference mode as a first test.

## 8. Analyse a trajectory without reading Rust

After any `sgrun orbit --out run.csv` (or a UI export):

```bash
python3 python/analysis/analyze_export.py run.csv --out analysis
```

This writes:

- `analysis.summary.txt` — row count, time span, max |dE_rel|, max |dL_rel|
- `analysis.energy.png` and `analysis.separation.png` (if matplotlib is installed)
- `analysis.model_vs_newton.txt` — rough diagnostic residual relative to a pure Newtonian expectation (meaningful only for thin V0 runs)

Install the optional plot dependency with `pip install matplotlib`. The script degrades gracefully if it is absent.

Example one-liner from a fresh build:

```bash
cargo run --release --locked -p shadowgrav-core --bin sgrun -- orbit --tau 1e-4 --steps 400 --orbits 2 --stride 20 --out orbit.csv
python3 python/analysis/analyze_export.py orbit.csv --out orbit_analysis
```

The energy plot for a well-resolved Verlet integration of a thin orbit should show a small periodic oscillation (typically 10⁻⁷–10⁻⁶ relative) rather than secular drift. Larger or growing residuals indicate either a non-conservative regime (V1/V2, overlap, α_bg > 0) or an integration problem.
