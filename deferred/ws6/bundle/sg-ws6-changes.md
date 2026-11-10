# CHANGES 20: WS6 independent validation (offline part)

| Change | Why | Evidence |
|---|---|---|
| `shadowgrav-core/examples/xcheck_dump.rs` (new) | Engine side of the cross-check: stdin geometry in, derived quantities and pair, total and correction forces out. Example only, not part of the library API. | built and run by `oracle/xcheck_random.py` |
| `oracle/xcheck_random.py` (new) | Second, independent evaluation of the V0 force law on 58 seeded random geometries in five classes, with its own error estimate | `report/data/ws6_xcheck_report.md`, `ws6_xcheck_rows.csv`, `ws6_xcheck_run.log` |
| `shadowgrav-core/tests/properties.rs` (new, 5 tests, std only) | Random-cluster covariance, collinear monotonicity and pair-sum bound, two-body reciprocity, hostile numbers, 9000 byte-mutated scenario files | `logs/test.log`; measured values in WS6_VALIDATION.md section 3 |
| `oracle/README.md`, `WS6_VALIDATION.md` | Method, results, limits, and what was not possible offline | |

No library source file was changed. `repro-hash` combined digest unchanged (`f2e108670c118a3d`). Test count 128 to 133 (the five new tests).
Not done here: proptest, cargo-fuzz, sanitizers, a second implementation from the Spec PDF (WS6_VALIDATION.md section 4).
