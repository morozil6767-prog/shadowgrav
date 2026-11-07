# Independent oracle (Python, mpmath/numpy)
Nothing here uses the Spec's overlap machinery (no HEALPix, no overlap graph, no pair closed form in the sky oracle).

| script | what it provides | used by |
|---|---|---|
| oracle_screening.py | exact R(tau) for test (c) (axisymmetric 1D integral, 30 digits). R(1)=0.877785076145848, R(5)=0.801867822859093, R(20)=0.800000002009972 (2.0e-9 above 0.8), R(50)=0.8 | tests/oracle_sky.rs `screening_curve_matches_exact_oracle` |
| oracle_sky.py | brute-force eq. 3 on a 1.6e7-point Fibonacci sphere for arbitrary 3D configs; noise ~6e-5 | tests/oracle_sky.rs (partial + 3D overlap, tau 1 and 5) |
| ../shadowgrav-core/tools/eq10_check.py | n-space integration check of eq. 10 | Addendum 2 item 4 |

Run: `python3 oracle_screening.py`; `python3 oracle_sky.py` (about 2 minutes). Requires numpy, mpmath.
sky_out.txt is the recorded output the Rust reference vectors were copied from.
