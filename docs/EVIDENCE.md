# Evidence for CLAIMS.md

This file maps every numbered claim in `CLAIMS.md` to a command, a toolchain, a commit, a log, and a result. A claim with no rustc 1.88.0 log is marked **UNSUPPORTED**. A matching `repro-hash` is a digest of this build. It is not a statement that the physical law is valid.

The bundle below was produced by `scripts/make-evidence.sh` and kept outside the repository.

## Bundle

| item | value |
|---|---|
| command | `./scripts/make-evidence.sh /tmp/sg-ws7c-evidence` |
| commit | `0a443e2e2eb289f1980feb2ad44c735549ea61df` |
| branch | `cursor/ws7-evidence-a7be` |
| parent code | `52058f4af342c8fa4443ea068d6295a4bbaa4efb` (`review/ws2c-ws6`); this commit adds the script and the test-only env note only |
| utc | `20261110T133231Z` (`version.txt`) |
| toolchain | `rustc 1.88.0 (6b00bc388 2025-06-23)`, `cargo 1.88.0 (873a06493 2025-05-10)` |
| script exit | `0` (`/tmp/sg-ws7c-evidence-run.log`, line `EXIT:0`) |
| directory | `/tmp/sg-ws7c-evidence` (`SHA256SUMS` in that directory) |

| step | command (via `rustup run 1.88.0`) | exit file | exit |
|---|---|---|---|
| fmt | `cargo fmt --all --check` | `fmt.exit` | 0 |
| clippy | `cargo clippy --locked --workspace --all-targets -- -D warnings` | `clippy.exit` | 0 |
| test | `cargo test --locked --workspace --all-targets` | `test.exit` | 0 |
| repro-hash | `cargo run --release --locked -p shadowgrav-core --bin sgrun -- repro-hash` | `repro-hash.exit` | 0 |
| spec-a | `cargo run --release --locked -p shadowgrav-core --bin verify -- spec-a` | `verify-spec-a.exit` | 0 |

`test.txt` has 24 harness lines. Summed: 142 passed, 0 failed, 4 ignored. Ignored names: `orbit_hundred_orbits`, `screening_three_body_short_evolution`, `certified_mode_vs_sharp_lens_oracle`, `screening_curve_certified_mode_matches_exact_oracle`. The test profile in the workspace `Cargo.toml` is `opt-level = 3`. This command was not `--release`.

`repro-hash.txt` (release, `parallel: true`, `vendored_libm: false`, linux, x86_64, 4 threads, `git_state: clean`):

```
orbit_forces       e60572a3d079862d
orbit_state_200    c643d780b40120fd
screen_std         9eee9748d2156d26
screen_val         5dd3fd3f4ebe8c39
overlap3d_std      2de33d701849ff93
orbit_v1           3b51b87fbd2ee11d
orbit_v2           35e7dd4558237473
combined           f2e108670c118a3d
```

`verify-spec-a.txt`: both PDF blocks exit through the process with `0 PDF criterion(s) failed`. At h=T/2000, 5 orbits, period relative error `3.349845e-6` (bound `5e-6`, PASS). At h=T/2000, 100 orbits, period relative error `3.290013e-6` (PASS). Addendum 2 bound `3.553058e-6` PASS on both. This run does not execute `verify c` or `sgrun certify`.

`SHADOWGRAV_VERIFY_PDF_STEPS` was unset for this bundle. The variable is test-only (README and `shadowgrav-core/VERIFY.md` on this branch): unset keeps h=T/2000; the value `1000` makes `verify` exit non-zero by design. This bundle did not run the `1000` path. The integration tests `default_t2000_exits_zero` and `t1000_scored_path_exits_nonzero_and_names_the_pdf_period_line` are part of the 142 and are `ok` in `test.txt`.

## Header of CLAIMS.md

The header says numerics were verified on the author's Linux and Windows runs, and that the egui UI, CI, and multi-OS reproducibility are not proven.

| statement | status |
|---|---|
| Linux numerics on rustc 1.88.0 at `0a443e2` | supported by the bundle above, this machine only |
| Windows numerics | **UNSUPPORTED**. No Windows log in this bundle |
| native egui window | **UNSUPPORTED**. Headless UI tests are in `test.txt`; no window capture is in this bundle |
| CI / multi-OS | **UNSUPPORTED**. `.github/workflows/ws4-matrix.yml` is `workflow_dispatch` only and was not run |

## Claims we make

### 1. Model scope

The engine source implements uniform spheres (`derive.rs` radius from mass and density), `A(tau)` (`derive.rs` `a_fun`), exterior closed form `a_eff/d^2` (`pair.rs` `pair_p`), overlap deficit `D_k = -expm1(-sum_j beta_j L_j)` (`reference.rs`), and V0/V1/V2 paths. That is source text, not a measurement.

Tests that passed in `/tmp/sg-ws7c-evidence/test.txt` (same command, toolchain, and commit as the bundle) and that exercise pieces of this claim:

| test | result in `test.txt` |
|---|---|
| `reference_reproduces_exact_screening_ratio` | ok |
| `reference_judges_the_production_quadrature` | ok |
| `v2_zero_velocity_reduces_to_v0` | ok |
| `v2_matches_eq10_thin_pair` | ok |
| `v2_orbit_converges_and_tracks_v1_at_large_vg` | ok |
| `v2_gating_rejects_large_eps_and_leaves_no_world` | ok |

No log in this bundle records a single measurement that the whole sentence (isotropic momentum flux, optional background extinction, V1 drag, and first-order V2 aberration together) matches a specification document. The frozen Spec PDF was not in the tree. **UNSUPPORTED** as a specification-conformance measurement. The passing tests above are implementation checks only.

### 2. Test (c), tau_c = 20

`CLAIMS.md` states measured values: standard `|R-0.8| ≈ 1.29e-3`, validation `≈ 1.58e-5`, fine/xhi `≈ 3.32e-6`, certified `|R-R_exact| ≈ 4.3e-7`.

Separate from the bundle above. Log `/tmp/sg-ws7d-meas.log`. Toolchain line in that log: `rustc 1.88.0 (6b00bc388 2025-06-23)`. `cargo 1.88.0 (873a06493 2025-05-10)`. Git head at the run: `2b7127db01ab5c3a2be691ce4d835509cbf7400d` (clean; the certify provenance prints `git_commit` `2b7127db01ab`). Commands and exits:

| command | exit |
|---|---|
| `rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin verify -- c` | 0 (`VERIFY_C_EXIT`) |
| `rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin sgrun -- certify` | 0 (`CERTIFY_EXIT`) |

`verify c` prints, at tau = 20: standard `|R-0.8|=1.29e-3`, validation `|R-0.8|=1.58e-5`, xhi `(0.002, 4e-4, cap14)` `|R-0.8|=3.32e-6`. Those three measured digits are **SUPPORTED**. `stage_c` in `verify.rs` prints them and does not call `rep.check`, so the log's `0 check(s) failed` is not a recorded pass of the `5e-3` clause. The clause itself is text. The printed standard value is `1.29e-3`.

`sgrun certify` prints `|R-R_exact|` `4.308e-7` at tau 20 (`4.307904e-7` in the JSON row), leaves `64767596`, cap hits 0, `pass_1e-6` true. The same command prints `4.742e-8` at tau 1 and `1.822e-7` at tau 5, both `pass_1e-6` true. The claim's `≈ 4.3e-7` on this geometry is **SUPPORTED** by the tau 20 row. `all_pass` is true.

The rim sentence "a few ×10⁻⁷" is the `-2.68e-7` cap shift in `WS3_QUADRATURE.md` (rustc 1.97.0). It is not in `/tmp/sg-ws7d-meas.log`. **UNSUPPORTED** on 1.88.0.

`artifacts/cli/verify-c.log` has the same standard, validation, and xhi digits and no rustc line. `artifacts/cli/certify.log` is rustc 1.83.0, `git_commit e89f0b1082a2`, dirty. Those older files are not the support for this row. The support is `/tmp/sg-ws7d-meas.log`.

`screening_curve_certified_mode_matches_exact_oracle` is ignored in the bundle's `test.txt`. The certify binary above is the run that scored certified test (c).

### 3. Independent oracles

`oracle/README.md` records `R(1)=0.877785076145848`, `R(5)=0.801867822859093`, `R(20)=0.800000002009972` (2.0e-9 above 0.8). `CLAIMS.md` writes exact `R(20) = 0.8 + 2.01e-9`. The README digit is `2.009972e-9` above 0.8. This bundle did not run `python3 oracle/oracle_screening.py`. Recomputation of the 30-digit oracle: **UNSUPPORTED**.

Production comparison tests that did run, same `test.txt`:

| test | result |
|---|---|
| `screening_curve_matches_exact_oracle` | ok |
| `screening_curve_graded_mode_matches_exact_oracle` | ok |
| `screening_curve_validation_mode_tracks_exact_oracle` | ok |
| `screening_curve_certified_mode_matches_exact_oracle` | ignored |
| `certified_mode_vs_sharp_lens_oracle` | ignored |

Passing rows compare the engine with constants stored in the tests. They do not recompute the Python oracle. Certified and sharp-lens certified comparisons: **UNSUPPORTED** in this run (ignored).

### 4. In-core reference mode (N ≤ 50)

`reference.rs` states the scope: Mode V0, `alpha_bg = 0`, bodies only, exterior pairs, N ≤ 50, same shadow integrand, not a Newtonian sum.

Passed in `test.txt`: `reference_two_body_matches_closed_form_pair`, `reference_refuses_out_of_scope_worlds`, `reference_reproduces_exact_screening_ratio`, `reference_judges_the_production_quadrature`, `reference_diag_out_of_scope_is_not_a_pass`, `reference_diag_reports_the_engine_error`.

Those tests support the path on the cases they construct. A sweep of every N ≤ 50 was not logged. **UNSUPPORTED** beyond the cases those tests build.

### 5. Parallel path

`parallel_hash_and_brute_are_bitwise_identical` is `ok` in `test.txt`. `repro-hash.txt` provenance has `"parallel": true`. Supported for the configurations that test builds, on this linux x86_64 release of the same commit. Not a statement about other CPU targets.

### 6. Conformance UX

Passed in `test.txt`:

| test | what the source asserts |
|---|---|
| `thin_fast_is_gated` | `perf_equiv.rs`: tau_c = 1 and mode V1 return `CoreError::Gating`; opening angle 0.9 returns `Config`; V0 with theta 0.5 is accepted |
| `sim::tests::sim_thread_runs_and_publishes_green_orbit` | `Badge::Green` |
| `sim::tests::bad_edit_is_rolled_back_and_flagged_red` | `Badge::Red` after a V2 edit with `alpha_bg = 0.1` |
| `sim::tests::load_preset_bumps_generation_and_pauses` | screening preset extras include the key `regime status` |

`app.rs` `badge_style` maps Green/Amber/Red to the strings `CONFORMING`, `CAVEATS`, `NON-CONFORMING`. No test in `test.txt` asserts those three strings, and this bundle has no screenshot. The painted words, including an amber `CAVEATS` run: **UNSUPPORTED**.

### 7. Reproducibility within a platform

Supported for this one platform by `repro-hash.txt`: combined `f2e108670c118a3d` on linux x86_64, rustc 1.88.0, commit `0a443e2`, default features (`vendored_libm: false`). Cross-OS identity and a `vendored-libm` build: **UNSUPPORTED**. See non-claim 6.

## Claims we do not make

These rows are limits written in `CLAIMS.md`. They are not measurements unless a log is named.

| # | non-claim | status on rustc 1.88.0 at `0a443e2` |
|---|---|---|
| 1 | Not Sakharov, Puthoff, GR, cosmology, or Lorentz-covariant physics | Text in `CLAIMS.md` and `MODEL_VS_OBSERVATION.md`. No command in this bundle tests those other theories. **UNSUPPORTED** as a measurement. |
| 2 | Standard, validation, and fine do not meet Spec 1e-6 on test (c); only certified does, on the stated geometry | `/tmp/sg-ws7d-meas.log`, rustc 1.88.0, commit `2b7127db01ab`. At tau 20, `verify c` prints `|R-0.8|` `1.29e-3` (standard), `1.58e-5` (validation), `3.32e-6` (xhi, f_v 0.002). Each printed value is above `1e-6`. `sgrun certify` sets `pass_1e-6` true at tau 1, 5, and 20 against `R_exact` (tau 20 error `4.307904e-7`). **SUPPORTED** for those presets on this geometry. Other geometries were not in that log. |
| 3 | Certified error is not rim-free; the next stair is not guaranteed to reach 1e-7 | Stated in `WS3_QUADRATURE.md` (cap-15 to cap-16 shift `-2.68e-7`). Not re-run here. **UNSUPPORTED** on 1.88.0. |
| 4 | V2 is not as thoroughly validated as V0/V1 and stays caveat-flagged | `v2_valid_flag_reports_fast_bodies_in_every_mode` is `ok` in `test.txt`. A comparison of V2 coverage against V0/V1 was not logged. The "less validated" sentence: **UNSUPPORTED** as a measurement. |
| 5 | Thin-fast forces are not scientifically interchangeable with conforming forces | `thin_fast_is_gated` is `ok`. `thin_fast_matches_conforming_within_barnes_hut_error` is `ok` and is a bounded match, not a measured scientific difference. A quantified non-interchangeability figure: **UNSUPPORTED**. Source labels the path non-conforming (`thin_fast.rs`). |
| 6 | Not bitwise identical across OS/CPU until multi-OS CI is green | No macOS or Windows `repro-hash` log, and `ws4-matrix.yml` was not dispatched. **UNSUPPORTED** as a cross-OS measurement. The non-claim is the absence of that run. |
| 7 | Not adopted by external research groups | No adoption log exists, and none is claimed. Text only. |

## Not in this bundle

- `cargo test --release`
- `verify` with `SHADOWGRAV_VERIFY_PDF_STEPS=1000` as a process (the integration test of that path is inside `test.txt`)
- `verify c` and `sgrun certify` are in `/tmp/sg-ws7d-meas.log`, not in this bundle's directory
- `python3 oracle/oracle_screening.py` and `oracle/xcheck_random.py`
- core MSRV `scripts/core-isolated-msrv.sh` on rustc 1.80.0
- cargo-deny, cargo-audit
- GitHub Actions
