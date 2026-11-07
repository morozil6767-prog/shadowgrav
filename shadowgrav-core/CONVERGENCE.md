# CONVERGENCE.md: quadrature error of the overlap correction, measured against independent oracles
All numbers come from files in this workspace: `report/data/convergence.txt` (produced by `bin/converge`, [S]), `report/data/scan_*.csv` (`sgrun scan`, [S]),
`oracle/` (Python, [P]). Evidence tags are defined in ADDENDUM3.md. Nothing in this file was produced by the session that wrote it except the [P] items, which I re-ran.

## 1. What is measured and against what

* **Test (c) geometry** (three equal collinear spheres, d = 10a, tau_c = 1, 5, 20): the reference is the **exact** R(tau) from `oracle/oracle_screening.py` (axisymmetric one-dimensional
  integral, mpmath, 30 digits). The scipy twin `oracle_screening_scipy.py` agrees to 1e-15 .. 4e-15 [P], and `screening_curve.csv` regenerates bit-for-bit [P]. R(20) = 0.8 + 2.01e-9.
  The exact curve is strictly decreasing on 2000 points of tau in [1e-3, 30] (maximum step -3.6e-14 .. negative everywhere) [P], tends to 1 with (1 - R)/tau = 0.1895 at tau = 1e-3 [P],
  and R - 0.8 decays like exp(-0.90 tau) for tau in [20, 30] [P].
* **Partial and 3D overlap**: two references, see section 3.

## 2. Test (c): error against refinement (tau-20 column is the Spec's test)

`|R - R_exact|` for the *standard* rows (first three, f_v = f_s = f) and the *validation-form* rows (the rest; f_s = 4e-4 throughout). "level" is the interior HEALPix level selected by f_v
(ADDENDUM3 item 3); identical leaf counts are the same computation.

### tau_c = 20

| f_v | f_s | cap | interior level | leaves | seconds | R | \|R - R_exact\| | error ratio to previous stair |
|---|---|---|---|---|---|---|---|---|
| 0.15 | 1e-01 | 12 | 5 | 1,232 | 0.00 | 0.798712472995 | 1.288e-03 |  |
| 0.075 | 7e-02 | 12 | 6 | 4,444 | 0.00 | 0.799701516093 | 2.985e-04 | 4.31 |
| 0.0375 | 4e-02 | 13 | 7 | 16,696 | 0.00 | 0.800141743651 | 1.417e-04 | 2.11 |
| 0.02 | 4e-04 | 14 | 8 | 540,512 | 0.15 | 0.799984164169 | 1.584e-05 | 8.95 |
| 0.01 | 4e-04 | 14 | 9 | 723,824 | 0.20 | 0.799988919810 | 1.108e-05 | 1.43 |
| 0.005 | 4e-04 | 14 | 10 | 1,467,932 | 0.35 | 0.799993376098 | 6.626e-06 | 1.67 |
| 0.002 | 4e-04 | 14 | 11 | 4,467,284 | 1.12 | 0.799996679851 | 3.322e-06 | 1.99 |
| 0.0015 | 4e-04 | 15 | 12 | 16,509,332 | 3.67 | 0.799998601767 | 1.400e-06 | 2.37 |
| 0.001 | 4e-04 | 15 | 12 | 16,509,332 | 3.63 | 0.799998601767 | 1.400e-06 |  |
| 0.0007 | 4e-04 | 15 | 13 | 64,767,596 | 13.71 | 0.799999571220 | 4.308e-07 | 3.25 |
| 0.0005 | 4e-04 | 15 | 13 | 64,767,596 | 14.11 | 0.799999571220 | 4.308e-07 |  |
| 0.0007 | 4e-04 | 16 | 13 | 64,767,596 | 14.48 | 0.799999571220 | 4.308e-07 |  |
| 0.0005 | 4e-04 | 16 | 13 | 64,767,596 | 14.02 | 0.799999571220 | 4.308e-07 |  |

### tau_c = 5

| f_v | f_s | cap | interior level | leaves | seconds | R | \|R - R_exact\| | error ratio to previous stair |
|---|---|---|---|---|---|---|---|---|
| 0.15 | 1e-01 | 12 | 5 | 1,232 | 0.00 | 0.801937862686 | 7.004e-05 |  |
| 0.075 | 7e-02 | 12 | 6 | 4,444 | 0.00 | 0.801675361497 | 1.925e-04 |  |
| 0.0375 | 4e-02 | 13 | 7 | 16,696 | 0.00 | 0.801919423367 | 5.160e-05 | 3.73 |
| 0.02 | 4e-04 | 14 | 8 | 540,512 | 0.15 | 0.801838994108 | 2.883e-05 | 1.79 |
| 0.01 | 4e-04 | 14 | 9 | 723,824 | 0.19 | 0.801855315414 | 1.251e-05 | 2.30 |
| 0.005 | 4e-04 | 14 | 10 | 1,467,932 | 0.35 | 0.801862700887 | 5.122e-06 | 2.44 |
| 0.002 | 4e-04 | 14 | 11 | 4,467,284 | 1.00 | 0.801865876326 | 1.947e-06 | 2.63 |
| 0.0015 | 4e-04 | 15 | 12 | 16,509,332 | 3.62 | 0.801867148735 | 6.741e-07 | 2.89 |
| 0.001 | 4e-04 | 15 | 12 | 16,509,332 | 3.83 | 0.801867148735 | 6.741e-07 |  |
| 0.0007 | 4e-04 | 15 | 13 | 64,767,596 | 14.33 | 0.801867640635 | 1.822e-07 | 3.70 |
| 0.0005 | 4e-04 | 15 | 13 | 64,767,596 | 14.21 | 0.801867640635 | 1.822e-07 |  |
| 0.0007 | 4e-04 | 16 | 13 | 64,767,596 | 13.81 | 0.801867640635 | 1.822e-07 |  |
| 0.0005 | 4e-04 | 16 | 13 | 64,767,596 | 13.95 | 0.801867640635 | 1.822e-07 |  |

### tau_c = 1

| f_v | f_s | cap | interior level | leaves | seconds | R | \|R - R_exact\| | error ratio to previous stair |
|---|---|---|---|---|---|---|---|---|
| 0.15 | 1e-01 | 12 | 5 | 1,232 | 0.00 | 0.877901171714 | 1.161e-04 |  |
| 0.075 | 7e-02 | 12 | 6 | 4,444 | 0.00 | 0.877725214697 | 5.986e-05 | 1.94 |
| 0.0375 | 4e-02 | 13 | 7 | 16,696 | 0.00 | 0.877797383906 | 1.231e-05 | 4.86 |
| 0.02 | 4e-04 | 14 | 8 | 540,512 | 0.14 | 0.877773213473 | 1.186e-05 | 1.04 |
| 0.01 | 4e-04 | 14 | 9 | 723,824 | 0.17 | 0.877780709466 | 4.367e-06 | 2.72 |
| 0.005 | 4e-04 | 14 | 10 | 1,467,932 | 0.34 | 0.877783489423 | 1.587e-06 | 2.75 |
| 0.002 | 4e-04 | 14 | 11 | 4,467,284 | 0.90 | 0.877784520673 | 5.555e-07 | 2.86 |
| 0.0015 | 4e-04 | 15 | 12 | 16,509,332 | 3.39 | 0.877784894277 | 1.819e-07 | 3.05 |
| 0.001 | 4e-04 | 15 | 12 | 16,509,332 | 3.52 | 0.877784894277 | 1.819e-07 |  |
| 0.0007 | 4e-04 | 15 | 13 | 64,767,596 | 13.38 | 0.877785028727 | 4.742e-08 | 3.84 |
| 0.0005 | 4e-04 | 15 | 13 | 64,767,596 | 12.78 | 0.877785028727 | 4.742e-08 |  |
| 0.0007 | 4e-04 | 16 | 13 | 64,767,596 | 12.95 | 0.877785028727 | 4.742e-08 |  |
| 0.0005 | 4e-04 | 16 | 13 | 64,767,596 | 14.14 | 0.877785028727 | 4.742e-08 |  |

Reading the table:

1. **Staircase.** Rows with equal leaf counts have equal R (f_v 0.0015 and 0.001; 7e-4 and 5e-4; cap 15 and 16). Refinement happens in whole HEALPix levels, each multiplying the leaves by about 4
   (3.7 then 3.9 at the top) and the time by about 3.9. ADDENDUM3 item 3 derives the leaf counts analytically (`oracle/stair_model.py`: 0.5% agreement at 64.8M leaves).
2. **Cheapest 1e-6 on this geometry.** tau = 20 first passes 1e-6 at the 64.8M-leaf stair (4.31e-7). The 16.5M stair is 1.40e-6, 40% above tolerance. There is no setting in between: the next
   cheaper setting is a different stair, not a coarser knob value.
3. **Order.** Least-squares slopes of log(error) on log(leaves) over all nine stairs are -0.70 (tau 20), -0.58 (tau 5) and -0.67 (tau 1). The ratio column shows that this is pre-asymptotic: the
   last two ratios are 2.4 -> 3.3, 2.9 -> 3.7 and 3.1 -> 3.8, rising toward 4 (second order in the interior). The Spec's O(h^3/2) is the silhouette-cell term; with the rim held at level 14 it is not the
   interior's rate. **Do not extrapolate a single power law** (the predicted leaves for 1e-7 range over 3e7 .. 9e8 across tau and fit windows).
4. **The first non-monotone entries are real.** At tau = 5 the error rises from 7.0e-5 (f = 0.15) to 1.9e-4 (f = 0.075) before falling: at coarse f the midpoint error changes sign between levels, so a
   single coarse run says little about accuracy.
5. **Rim floor, unresolved.** See ADDENDUM3 item 4 and `verify -- rim`.

### Errors across the whole curve (64 optical depths, tau in [1e-3, 200]; `report/data/scan_*.csv`, compared with the exact curve [P])

| mode | max \|R - R_exact\| | at tau |
|---|---|---|
| standard (f = 0.15) | 3.27e-3 | 200 |
| validation (f_v 0.02) | 2.96e-5 | 7.3 |
| xhi (f_v 0.002) | 3.32e-6 | 20 |
| certified (f_v 7e-4), 8 points | 4.31e-7 | 20 |

For xhi the worst tau is exactly the Spec's tau = 20, so the preset that looked best at the Spec's test point is the worst point of its own curve; this is why a single-tau claim was fragile.

## 3. Partial and 3D overlap (non-nested discs)

Four configurations (tau_c = 1 and 5; third sphere at (12, 2, 0)a and (12, 1.4, 1.4)a; target body 0 sees B at 10a and C at about 12.2a with partially overlapping discs).

### 3.1 Two oracles

| oracle | method | accuracy |
|---|---|---|
| Fibonacci sky grid (`oracle/oracle_sky.py`) | brute-force eq. 3 on 1.6e7 points, no Spec machinery | |dF|/|F| within 4.9e-6 .. 1.7e-5 and Delta^NA within 0.05% .. 0.22% of the sharp oracle [P] (its stated noise was 6e-5) |
| **sharp lens oracle** (`oracle/oracle_lens.py`, new) | Corr = - integral over the lens of n D_B D_C dOmega (exact for two exterior occluders), nested adaptive quadrature in two parametrisations | the two parametrisations agree to 1e-17; a dense midpoint grid differs by 2.4e-4, 1.1e-4, 3.9e-5 (N = 500, 1000, 2000), the O(h^1.5) rate expected for square-root edges [P] |

Sharp reference values (F_k / Gamma_k for the three bodies and Delta^NA of body 0) are in `oracle/lens_reference.json` and, as constants, in `tests/oracle_lens.rs`.

### 3.2 The core against them ([S], against the Fibonacci grid)

| mode | max \|dF\|/\|F\| | Delta^NA relative error | Spec 4.3 target 1e-4 |
|---|---|---|---|
| standard (f 0.15, about 55 leaves) | 9.6e-4 | up to 8.6% (largest at tau = 5) | not met |
| validation (f_v 0.02) | 3.7e-5 | 0.42% | met (also after adding the oracle's own 1.7e-5) |

Standard mode is therefore a coarse quadrature on partial overlap and is described that way in the UI. `tests/oracle_sky.rs` bounds are regression bounds (about 1.5x the measurement);
`tests/oracle_lens.rs` repeats them against the sharp oracle and adds the explicit 1e-4 assertion for validation mode. The true errors against the sharp oracle have **not** been measured yet
(no Rust here); the first run of `cargo test --release -p shadowgrav-core --test oracle_lens -- --nocapture` prints them.

## 4. Cost model

* Leaves for a nested pair: 2 Omega / A_l (ADDENDUM3 item 3); time about 0.2 microseconds per leaf in the measured serial build (64.8M leaves in 14 s). A stair is 4x leaves, 3.9x time.
* This is the cost of **one evaluation of one overlapping configuration**; a certified run of an interactive simulation (steps x evaluations) is out of reach, which is why validation mode exists for dynamics
  and certified mode for single evaluations that support a quoted number.

## 5. What would settle the open questions

| question | experiment | cost |
|---|---|---|
| Is the xhi 6.96e-7 a recording error? | `verify -- c` on the owner's machine | seconds |
| Rim floor or interior-limited at the certified stair? | `verify -- rim` (2 x 2 grid f_v x f_s) | about 1 minute |
| True errors of standard/validation on partial overlap | `cargo test -p shadowgrav-core --test oracle_lens -- --nocapture` | seconds |
| Does the certified preset hold off the collinear geometry? | `oracle_lens` ignored test `certified_mode_vs_sharp_lens_oracle` | unmeasured |
| Do results depend on platform? | CI reproducibility job (`sgrun repro-hash`, vendored libm) | CI |

## 6. Reproduction

    cargo run --release -p shadowgrav-core --bin converge            # about 1 minute; regenerates convergence.txt
    cargo run --release -p shadowgrav-core --bin sgrun -- certify    # about 40 s
    python3 oracle/oracle_screening_scipy.py oracle/screening_curve.csv
    python3 oracle/oracle_lens.py --write oracle/lens_reference.json
    python3 oracle/stair_model.py
    python3 report/make_figures.py
