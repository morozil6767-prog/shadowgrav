# Cross-check report (seed 20261107, 58 geometries, oracle grid (192, 768, 96) and (384, 1536, 192))

Oracle self-estimate (max over geometries of |fine - coarse| / gross, gross = Int D dOmega): 4.71e-05

Maximum discrepancy per quantity, relative (derived: a, beta, tau_c, A_eff; pair: closed form vs direct 1-D integral; total: |F_engine - F_oracle| / gross with gross = Int D dOmega, the cancellation-free scale; corr = total - pair on the same scale; `total_rel` = relative to |F| itself, shown for geometries with |F|/gross >= 0.05 only). `refused` = engine returned an error.

| class | tier | geometries | refused | derived | pair | total | corr | total/abs(F), cond>=0.05 | beyond oracle noise | max leaves | cap hits |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A sparse | standard | 10 | 0 | 2.8e-15 | 1.4e-15 | 2.0e-13 | 2.0e-13 | 2.1e-13 | 0 | 0 | 0 |
| A sparse | validation | 10 | 0 | 2.8e-15 | 1.4e-15 | 2.0e-13 | 2.0e-13 | 2.1e-13 | 0 | 0 | 0 |
| A sparse | graded | 10 | 0 | 2.8e-15 | 1.4e-15 | 2.0e-13 | 2.0e-13 | 2.1e-13 | 0 | 0 | 0 |
| A sparse | certified | 2 | 0 | 2.8e-15 | 1.3e-15 | 2.0e-13 | 2.0e-13 | 2.1e-13 | 0 | 0 | 0 |
| B partial overlap | standard | 12 | 0 | 2.8e-15 | 1.5e-15 | 1.8e-03 | 1.8e-03 | 3.6e-03 | 5 | 2,701 | 0 |
| B partial overlap | validation | 12 | 0 | 2.8e-15 | 1.5e-15 | 3.6e-05 | 3.6e-05 | 5.1e-05 | 5 | 1,043,769 | 0 |
| B partial overlap | graded | 12 | 0 | 2.8e-15 | 1.5e-15 | 1.3e-06 | 1.3e-06 | 1.3e-06 | 0 | 31,018,815 | 0 |
| B partial overlap | certified | 2 | 0 | 6.0e-16 | 4.7e-16 | 6.7e-14 | 6.7e-14 | 7.5e-14 | 0 | 0 | 0 |
| C deep overlap | standard | 12 | 0 | 2.8e-15 | 9.3e-15 | 2.6e-03 | 2.6e-03 | 8.6e-03 | 12 | 9,686 | 0 |
| C deep overlap | validation | 12 | 0 | 2.8e-15 | 9.3e-15 | 1.6e-04 | 1.6e-04 | 2.4e-04 | 7 | 3,625,716 | 0 |
| C deep overlap | graded | 12 | 0 | 2.8e-15 | 9.3e-15 | 6.5e-06 | 6.5e-06 | 1.5e-05 | 0 | 70,202,781 | 0 |
| C deep overlap | certified | 2 | 0 | 2.8e-15 | 1.0e-15 | 2.2e-06 | 2.2e-06 | 2.2e-06 | 0 | 161,950,874 | 0 |
| D near contact | standard | 12 | 0 | 3.0e-15 | 1.8e-15 | 7.1e-03 | 7.1e-03 | 7.1e-03 | 12 | 2,676 | 0 |
| D near contact | validation | 12 | 0 | 3.0e-15 | 1.8e-15 | 8.8e-05 | 8.8e-05 | 8.8e-05 | 11 | 829,188 | 0 |
| D near contact | graded | 12 | 0 | 3.0e-15 | 1.8e-15 | 3.8e-06 | 3.8e-06 | 4.0e-06 | 0 | 23,109,093 | 0 |
| D near contact | certified | 2 | 0 | 2.8e-15 | 1.8e-15 | 1.8e-06 | 1.8e-06 | 1.9e-06 | 0 | 50,494,342 | 0 |
| E interior | standard | 12 | 0 | 2.9e-15 | 5.1e-15 | 1.2e-03 | 1.2e-03 | 1.2e-03 | 12 | 5,738 | 0 |
| E interior | validation | 12 | 0 | 2.9e-15 | 5.1e-15 | 1.1e-04 | 1.1e-04 | 1.4e-04 | 5 | 1,417,539 | 0 |
| E interior | graded | 12 | 0 | 2.9e-15 | 5.1e-15 | 2.1e-05 | 2.1e-05 | 1.4e-04 | 1 | 52,817,141 | 0 |
| E interior | certified | 2 | 0 | 2.8e-15 | 1.6e-15 | 2.9e-06 | 2.9e-06 | 2.5e-05 | 0 | 49,415,934 | 0 |
| ALL | standard | 58 | 0 | 3.0e-15 | 9.3e-15 | 7.1e-03 | 7.1e-03 | 8.6e-03 | 41 | 9,686 | 0 |
| ALL | validation | 58 | 0 | 3.0e-15 | 9.3e-15 | 1.6e-04 | 1.6e-04 | 2.4e-04 | 28 | 3,625,716 | 0 |
| ALL | graded | 58 | 0 | 3.0e-15 | 9.3e-15 | 2.1e-05 | 2.1e-05 | 1.4e-04 | 1 | 70,202,781 | 0 |
| ALL | certified | 10 | 0 | 2.8e-15 | 1.8e-15 | 2.9e-06 | 2.9e-06 | 2.5e-05 | 0 | 161,950,874 | 0 |

Worst three geometries per tier (total):
- standard: geometry 37 (D near contact, n=3, tau0=20.0): total 7.08e-03, oracle err 1.8e-05, |F|/gross 8.4e-01
- standard: geometry 23 (C deep overlap, n=5, tau0=20.0): total 2.63e-03, oracle err 1.4e-05, |F|/gross 1.1e-01
- standard: geometry 45 (D near contact, n=3, tau0=20.0): total 2.49e-03, oracle err 7.6e-06, |F|/gross 8.7e-01
- validation: geometry 26 (C deep overlap, n=5, tau0=5.0): total 1.59e-04, oracle err 1.6e-05, |F|/gross 2.1e-01
- validation: geometry 52 (E interior, n=4, tau0=5.0): total 1.08e-04, oracle err 4.7e-05, |F|/gross 9.4e-03
- validation: geometry 48 (E interior, n=3, tau0=5.0): total 9.70e-05, oracle err 2.7e-05, |F|/gross 1.1e-01
- graded: geometry 57 (E interior, n=3, tau0=20.0): total 2.11e-05, oracle err 4.4e-05, |F|/gross 1.8e-03
- graded: geometry 49 (E interior, n=4, tau0=20.0): total 1.06e-05, oracle err 1.9e-05, |F|/gross 7.3e-06
- graded: geometry 56 (E interior, n=3, tau0=5.0): total 7.20e-06, oracle err 1.1e-05, |F|/gross 5.3e-02
- certified: geometry 48 (E interior, n=3, tau0=5.0): total 2.87e-06, oracle err 2.7e-05, |F|/gross 1.1e-01
- certified: geometry 30 (C deep overlap, n=3, tau0=5.0): total 2.19e-06, oracle err 1.0e-05, |F|/gross 6.5e-01
- certified: geometry 36 (D near contact, n=3, tau0=5.0): total 1.75e-06, oracle err 3.5e-06, |F|/gross 8.9e-01
