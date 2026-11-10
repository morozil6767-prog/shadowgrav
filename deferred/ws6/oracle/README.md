# Independent oracle (Python, mpmath/numpy)
Nothing here uses the Spec's overlap machinery (no HEALPix, no overlap graph, no pair closed form in the sky oracle).

| script | what it provides | used by |
|---|---|---|
| oracle_screening.py | exact R(tau) for test (c) (axisymmetric 1D integral, 30 digits). R(1)=0.877785076145848, R(5)=0.801867822859093, R(20)=0.800000002009972 (2.0e-9 above 0.8), R(50)=0.8 | tests/oracle_sky.rs `screening_curve_matches_exact_oracle` |
| oracle_sky.py | brute-force eq. 3 on a 1.6e7-point Fibonacci sphere for arbitrary 3D configs; noise ~6e-5 | tests/oracle_sky.rs (partial + 3D overlap, tau 1 and 5) |
| ../shadowgrav-core/tools/eq10_check.py | n-space integration check of eq. 10 | Addendum 2 item 4 |

Run: `python3 oracle_screening.py`; `python3 oracle_sky.py` (about 2 minutes). Requires numpy, mpmath.
sky_out.txt is the recorded output the Rust reference vectors were copied from.

## WS6 cross-check (second implementation on random geometries)
| script | what it provides |
|---|---|
| xcheck_random.py | 58 seeded random geometries (A sparse, B partial overlap, C deep overlap, D near contact, E target inside a sphere). Independent Python evaluation of F = Gamma * Int n D dOmega (Gauss-Legendre in sin-substituted polar coordinates about each occluder, chord-weight sharing) and of the pair force (direct 1-D integral), compared with the engine at tiers standard, validation, graded and (every 6th geometry) certified. Own error estimate from a doubled grid. |
| ../shadowgrav-core/examples/xcheck_dump.rs | The engine side: reads a geometry on stdin, prints the engine's derived quantities, pair, total and correction forces. |

Run (about 7 minutes): `cargo build --release -p shadowgrav-core --example xcheck_dump` then
`python3 -I oracle/xcheck_random.py --dump target/release/examples/xcheck_dump --out-prefix xc`. Needs numpy and scipy (no mpmath).
Recorded output: report/data/ws6_xcheck_report.md, ws6_xcheck_rows.csv. Method, limits and results: WS6_VALIDATION.md.
