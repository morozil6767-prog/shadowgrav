# Shadowgrav baseline audit

Date: 2026-11-07. Host: Ubuntu 24.04.5 LTS, Linux 6.12.113+, x86_64, 4-core Intel Xeon. This is a build and test record of the supplied archive. It is not a feature change, a release, or a claim that the classical shadow-gravity model matches nature.

The imported tree was not edited to make tests pass. After every run, all 131 extracted source files still matched the archive byte for byte.

## What was imported

| Input | Identity |
|---|---|
| Archive | `artifacts/inputs/source.tar.gz`, SHA-256 `093498b91580f5f4a8aa43fa8e306d9e8985b93e24bb6c387a543aabd61fa0f4`, 151 tar entries, 131 files under `shadowgrav/` |
| Reference | `artifacts/inputs/reference.pdf`, SHA-256 `0954597eaddb66d38244bea7ddd2ab7c876af1829c7399d387d1dd7a67217375` |
| Per-file hashes | `artifacts/inputs/extracted-tree.sha256` |

Included versions, from the crate manifests (there is no git tag and `publish = false`):

- `shadowgrav-core` 0.2.1, edition 2021, `rust-version` 1.70
- `shadowgrav-ui` 0.1.0, edition 2021, `rust-version` 1.76, dependency `eframe` 0.29
- Python package `shadowgrav` 0.1.0, standard library only; NumPy is optional
- Prose in `CLAIMS.md` calls this a v1.0 candidate. That sentence is a document, not a tag.

The archive has no `.git` directory. Provenance strings stamped by `build.rs` therefore show this scratch repository's seed commit `e89f0b1082a2`, and `git_state` `dirty`, because the working tree was untracked at build time. That stamp is not an upstream Shadowgrav revision.

## What reference.pdf is

`reference.pdf` is the 15-page document titled *Mathematical Specification: Non-Additive Shadow-Gravity N-Body Model*, dated Nov 1, 2026, with the byline printed as `@Mohammed Rozil`. pypdf reads 15 pages; the footers say "Page N of 15". Extracted text is `artifacts/inputs/reference.pdf.txt`.

It is the frozen mathematical contract the repository says it implements (force law, quadrature, tests (a)–(e)). It is user-supplied specification text, not an observational dataset and not a peer review. Section 7 states numerical conformance tests for that model. Section 8 says the model is static, non-relativistic, and geometric-optics, and that it must not be used to claim general-relativistic effects. `MODEL_VS_OBSERVATION.md` says the same thing in the repository's own words. Agreement with section 7 is a statement about this implementation of this model. It is not evidence that Le Sage shadow gravity is the gravity of the solar system.

The repository also contains `ADDENDUM2.md` and `ADDENDUM3.md`, which propose tolerance changes that are not in the PDF. Where those disagree, both numbers are reported below.

## Toolchains

| Role | Compiler | Result |
|---|---|---|
| Preinstalled default | rustc 1.83.0 (90b35a623 2024-11-26), cargo 1.83.0 | Core library, tests, and `sgrun` build and pass. UI and `cargo test --workspace` do not. |
| Installed with rustup, not made the default | rustc 1.99.1 (58cf8f9c3 2026-10-13) | Workspace tests, UI tests, UI binary, and `sgrun repro-hash` succeed. This is the compiler named in `REPRODUCIBILITY.md`. |
| Python | 3.12.3 | Wrapper tests. Oracle venv: NumPy 2.5.4, SciPy 1.18.1, mpmath 1.4.1, Matplotlib 3.11.2. |

The UI lockfile resolves `calloop` 0.14.5, whose manifest is `edition = "2024"`. Cargo 1.83.0 stops with exit 101:

```
error: failed to parse manifest at `.../calloop-0.14.5/Cargo.toml`
feature `edition2024` is required
```

The crate's own `rust-version = "1.76"` is not sufficient for this lockfile. No manifest was edited.

Core dependencies actually used: `rayon` 1.12 (default feature `parallel`), optional `libm` 0.2 (`vendored-libm`, off by default). UI: `eframe` 0.29, which pulls egui, wgpu, and glow.

## Commands, exit codes, and counts

Default `cargo test` counts. Ignored tests are listed after the table. Logs are under `artifacts/logs/`.

| Command | Toolchain | Exit | Passed | Failed | Ignored |
|---|---|---:|---:|---:|---:|
| `cargo test --release --locked -p shadowgrav-core -- --test-threads=4` | 1.83.0 | 0 | 122 | 0 | 4 |
| `cargo test --release --locked -p shadowgrav-core --no-default-features --features vendored-libm -- --test-threads=1` | 1.83.0 | 0 | 122 | 0 | 4 |
| `cargo test --release --locked --workspace` | 1.83.0 | 101 | — | — | — |
| `cargo test --release --locked -p shadowgrav-ui` | 1.83.0 | 101 | — | — | — |
| `cargo test --release --locked -p shadowgrav-ui` | 1.99.1 | 0 | 7 | 0 | 0 |
| `cargo build --release --locked -p shadowgrav-ui` | 1.99.1 | 0 | binary produced | — | 20 warnings |
| `cargo test --release --locked --workspace` | 1.99.1 | 0 | 129 | 0 | 4 |
| `python3 -m unittest discover -s python/tests -v` | 3.12.3 | 0 | 7 | 0 | 0 |

The 129 workspace passes on 1.99.1 are the 122 core tests plus the 7 UI tests. The same 4 core tests stay ignored unless requested. Wall clock for the 1.83 core run, including the release compile, was 01:57:40Z–01:58:01Z. The certified collinear case inside that run is part of `integrator` (suite time 6.19 s). The separate `sgrun certify` run took about 17 s for three 64,767,596-leaf evaluations (5.52 s, 5.87 s, 5.88 s).

The four ignored tests were then run on 1.83.0. All passed:

| Test | Exit | Time |
|---|---:|---|
| `integrator` `--ignored` (`orbit_hundred_orbits`, `screening_three_body_short_evolution`) | 0 | 8.08 s |
| `oracle_sky` `--ignored` (`screening_curve_certified_mode_matches_exact_oracle`) | 0 | 29.62 s |
| `oracle_lens` `--ignored` (`certified_mode_vs_sharp_lens_oracle`) | 0 | 0.91 s |

`CONVERGENCE.md` still describes the partial-overlap certified comparison as unmeasured. This run measured it. The assertion passed. Example lines from `artifacts/logs/oracle-lens-ignored.txt`: certified partial overlap, max `|dF|/|F|` about 2.7e-8 (tau 1) and 1.5e-7 (tau 5), with about 1.0e6–1.1e6 leaves. That is a code-versus-oracle residual for those four geometries, not a new physical claim.

UI build warnings on 1.99.1: 20, including `f32` fallbacks that rustc says will become a hard error (issue 154024) and one unused `conformance_summary`. The build still exited 0. Nothing was silenced or rewritten.

Every CLI invocation in `artifacts/cli/exit-codes.txt` exited 0 (24 lines): `provenance`, `repro-hash` twice, validate and run for all six scenario files, thin orbit, tutorial orbit, drag, validation and standard scans, skymap, `verify`, `verify c`, and `certify`.

## Numerical comparison

Engine numbers below are from the 1.83.0 default build (`parallel` on, `vendored-libm` off, 4 threads). They match the same `sgrun repro-hash` on 1.99.1.

### Test (a), thin two-body orbit

`verify` with no arguments, 5 orbits, h = T/1000, tau_c = 1e-6:

| Check | Measured | PDF section 7 | This run against the PDF | Repository check |
|---|---:|---|---|---|
| Period relative error | 1.34e-5 | within 5e-6 | not met (1.34e-5 > 5e-6) | met against Addendum 2's `0.36 (ωh)^2` bound of 1.42e-5 |
| max \|ΔE/E\| | 3.90e-10 | at most (ωh)^2 ≈ 4e-5 | met (bound used by the binary: 3.95e-5) | met |
| max \|ΔL/L\| | 5.18e-15 | at most 1e-12 | met | met |
| R_P | 0 | at most 1e-12 | met | met |
| Δ^NA | 0 | 0 | met | met |

The PDF asks for 100 orbits. The default driver uses 5. The ignored 100-orbit test passed the repository's Addendum 2 assertion. It does not print a separate period error, so this audit does not claim the 100-orbit error is under 5e-6.

Saturation table from the same `verify` run, against the PDF's five-decimal values: S(0.1) = 0.963480 vs 0.96348, S(1) = 0.707277 vs 0.70728, S(10) = 0.147001 vs 0.147. The binary's own tolerance is 6e-6 and those checks passed.

### Test (b) / thin-limit force, tutorial check A

`sgrun run` on `two_body_thin.json`:

- Engine F on body 0: 100.07463492138328, which rounds to the tutorial's 100.07463
- Independent Newtonian hand value `1/(10 a)^2` with `a = (3/(4 π 2.39e5))^(1/3) = 0.009996266592921414`: 100.07470997738169, which rounds to the tutorial's 100.07471
- Absolute gap: 7.5056e-5. Relative gap: 7.500e-7, equal to the run's `delta_sat`
- `TUTORIAL.md` says those two forces "differ by 7.5e-6". That figure is neither the absolute gap nor the relative gap. The two quoted force values match this run. The stated difference does not.

### Test (c), collinear screening

Exact R from a fresh `oracle/oracle_screening.py` (mpmath, 30 digits), identical to the constants compiled into `sgrun`:

| tau_c | R_exact |
|---:|---:|
| 1 | 0.877785076145848 |
| 5 | 0.801867822859093 |
| 20 | 0.800000002009972 (0.8 + 2.01e-9) |

SciPy twin minus those values: 1.0e-15, 6.7e-16, 3.8e-15. The regenerated `screening_curve.csv` is byte-identical to both shipped copies.

`sgrun certify` (certified preset f_v = 7e-4, f_s = 4e-4, cap 15), exit 0, `all_pass` true:

| tau | R | \|R − R_exact\| | Leaves | PDF \|R−0.8\| ≤ 1e-6 |
|---:|---:|---:|---:|---|
| 1 | 0.877785028726727 | 4.742e-8 | 64,767,596 | met |
| 5 | 0.801867640635352 | 1.822e-7 | 64,767,596 | met |
| 20 | 0.799999571219568 | 4.308e-7 | 64,767,596 | met |

These are the same digits as `CONVERGENCE.md` and `CLAIMS.md` (4.7e-8, 1.8e-7, 4.3e-7 at the certified stair).

`verify c` at tau = 20, which compares with 0.8 rather than R_exact:

| Tier | \|R − 0.8\| | PDF 1e-6 | Project's own tier statement |
|---|---:|---|---|
| Standard f = 0.15 | 1.29e-3 | not met | within the provisional 5e-3 in `CLAIMS.md` |
| Validation f_v = 0.02 | 1.58e-5 | not met | matches the documented validation measurement |
| xhi f_v = 0.002 | 3.32e-6 | not met | matches the documented xhi measurement; the old 6.96e-7 figure remains superseded inside the repo |
| Certified | 4.31e-7 vs R_exact | met on this geometry | matches the documented certified measurement |

Validation-tier scan, rounded to six decimals, matches `TUTORIAL.md`: 0.01 → 0.998113, 0.1 → 0.981916, 1 → 0.877773, 5 → 0.801839, 20 → 0.799984. Errors against R_exact at tau 1, 5, and 20 are 1.186e-5, 2.883e-5, and 1.584e-5, matching the convergence table.

### Test (d)

`verify` drag, h·γ = 0.005: fitted γ relative error 2.09e-6, under the PDF's 1e-5. Frame invariance 4.6e-14, under 1e-12. `sgrun drag` reports the same relative error 2.093e-6. `eq10_check.py` reproduced Addendum 2's second-order coefficients 0.109, 0.142, and 0.157 at the three scales. That supports the repository's first-order aberration check. `CLAIMS.md` still says V2 is less exercised than V0/V1; this audit does not upgrade that sentence.

### Reproducibility digest

`sgrun repro-hash`, default features, combined `f2e108670c118a3d` on both rustc 1.83.0 and rustc 1.99.1, twice in a row on 1.83.0. Individual digests match `REPRODUCIBILITY.md` line for line. This machine has 4 threads; the document's provenance line says 2. The digest still matched.

Serial `vendored-libm` on 1.83.0 produced a different combined digest, `064448c38b42b9f2`. `screen_val` and `orbit_v2` changed; the other five case digests did not. One Linux vendored digest cannot show cross-OS identity. macOS and Windows were not run. `.github/workflows/ci.yml` was read and not executed. No Actions logs were in the archive.

## GUI: build versus runtime

Headless UI tests (7) passing does not open a window. The release binary was then built with rustc 1.99.1 (`/tmp/sg-stable/release/shadowgrav-ui`, not left in the git tree). `ldd` reported no missing libraries.

Runtime, on this machine only: `DISPLAY=:1`, X.Org 21.1.11, 1920×1200, no `/dev/dri`. The process stayed up, mapped a viewable window titled `shadowgrav — non-additive shadow-gravity research instrument`, and painted a first frame. `xwininfo` recorded 1484×891 at +1+57 (the source asks for 1500×900). The frame shows preset (a), mode V0, standard quadrature, a green CONFORMING badge, two bodies, a force arrow, and the sky panel. The visible ω and period (1.414213032 and 4.442884604) match `sgrun orbit` for tau_c = 1e-6. Orbits completed stayed at 0, which is the paused startup state. stderr was empty (`artifacts/gui/launch.log`).

The audit harness then sent SIGTERM. That is not a clean quit and not a crash. Screenshots, taken from the live display:

- `artifacts/gui/window.png` (the client window)
- `artifacts/gui/root.png` (full desktop, so the window is shown on this session)

Not done: Play, Step, preset (b) or (c), export, thin-fast, resize, or the rest of the `MACHINE_PROOF.md` checklist. This is not a macOS or Windows GUI result. Nothing here was drawn by hand.

## Python and oracles

`python/tests/test_basic.py` ran 7 tests in 0.545 s, exit 0, using `SHADOWGRAV_SGRUN` pointed at the 1.83.0 binary. The package was not pip-installed; the test file inserts its own path. `CHANGES_15` says "Python 6 pass". The file in this archive contains 7 tests, and all 7 passed.

`analyze_export.py` on the tutorial orbit (tau_c = 1e-4, 800 steps, 2 orbits) wrote a summary and two plots. Max |dE/E| = 1.521e-8, max |dL/L| = 2.7e-15. The tutorial's "typically 1e-7 to 1e-6" describes a larger band than this particular run.

`stair_model.py` exits 1 if the current directory is the repository root (`FileNotFoundError` on `../report/data/convergence.txt`). Run from `oracle/`, it exits 0. At the certified stair the interior-leaf model is within 0.5% of the measured 64,767,596 leaves (ratio 1.005), which is the agreement `CONVERGENCE.md` already states.

`oracle_lens.py` exited 0 and reported parameter-spread about 1e-17, with SciPy `IntegrationWarning`s about roundoff. It was not given `--write`, so `oracle/lens_reference.json` was not replaced.

Not re-run: `oracle/oracle_sky.py` (1.6e7-point Fibonacci), `oracle/oracle_unequal_radius.py`, and `report/make_figures.py`. The unequal-radius budgets in `ERROR_BUDGETS.md` were not regenerated. Rust tests that compare with the stored sky and lens references did pass.

## Environment workarounds

These are outside the source tree:

1. Apt packages required to link the Linux UI, matching `.github/workflows/ci.yml`: X11, xcb, xkbcommon, wayland, GL, EGL, udev, plus ImageMagick, scrot, and xdotool for the screenshot.
2. `rustup toolchain install stable` provided rustc 1.99.1. The default toolchain was left at 1.83.0. UI and the 1.99.1 workspace used `rustup run stable` and `CARGO_TARGET_DIR=/tmp/sg-stable`.
3. A virtualenv under `/tmp/sg-venv` supplied NumPy, SciPy, mpmath, and Matplotlib. The unittest run used system Python.
4. Vendored-libm tests used `CARGO_TARGET_DIR=/tmp/sg-vendored` so they would not replace the default `sgrun`.

`scripts/local-proof.sh` was inspected and not used as the collector. After the UI build it runs `sgrun repro-hash` and then stores `PIPESTATUS[0]` as `ui_build_exit`, so a failed UI build can be recorded as success when the hash command succeeds. The exit codes in this audit come from the individual commands, not from that script.

`build.rs` labels any non-empty `git status --porcelain` as dirty, and an empty status becomes the string `clean-or-unknown` rather than `clean`. Observed, not changed.

## What this does not show

- No macOS or Windows compile, test, hash, or screenshot.
- No GitHub Actions run. A workflow file is not a green CI result.
- Cross-OS bitwise identity is open. The vendored digest here already differs from the default digest on one Linux host.
- The PDF period tolerance of 5e-6 is not met by the 5-orbit measurement. The code tests a looser addendum.
- Standard, validation, and xhi tiers do not meet the PDF's 1e-6 test (c) gate. Certified does, on the collinear equal-sphere geometry that was run.
- Native GUI behavior beyond the startup frame.
- `oracle_sky.py` and `oracle_unequal_radius.py` were not regenerated.
- No physical endorsement. The model, by its own specification, has no light deflection, no gravitational waves, and a drag term that general relativity does not have.

## Next steps, in the order they change the claim

1. Decide whether test (a) is judged by the PDF's 5e-6 or by Addendum 2. This run does not satisfy both.
2. Keep quoting test (c) only with the quadrature tier. Certified met 1e-6 here. The cheaper tiers did not.
3. Walk the `MACHINE_PROOF.md` checklist on a real session (play, presets, export, thin-fast badge) before calling the UI visually verified.
4. Run the existing CI workflow on macOS and Windows, and compare vendored-libm digests, before saying the result is cross-platform.
5. If documents are corrected later, fix the tutorial's 7.5e-6 difference, the `local-proof.sh` exit status, and the `stair_model.py` relative path. Those were left as they shipped.
6. UI warnings on rustc 1.99.1, especially the `f32` fallback scheduled to become a hard error, will break a future compiler even though this build exited 0.

## Where the evidence is

| Path | Contents |
|---|---|
| `artifacts/AUDIT.md` | this summary |
| `artifacts/results.json` | machine-readable counts, hashes, and the numerical gaps |
| `artifacts/inputs/` | original archive, PDF, extracted text, SHA-256 |
| `artifacts/logs/` | cargo logs and exit codes |
| `artifacts/cli/` | `sgrun` and `verify` transcripts |
| `artifacts/python/` | unittest log, orbit summary, plots |
| `artifacts/oracle/` | mpmath, SciPy, lens, stair, eq. 10 |
| `artifacts/gui/window.png` | startup screenshot of the native window |
| `artifacts/gui/root.png` | same window on the desktop |
