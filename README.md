# shadowgrav

**Classical non-additive shadow-gravity (Le Sage-type) N-body research instrument**

A binary64 reference engine and native desktop instrument for testing the *specified classical shadow model*. This is not general relativity, Sakharov induced gravity, cosmology, or a validated theory of gravity.

| Layer | Repository state | Evidence status |
|---|---|---|
| Physics core | Pair-exact force path, finite-sphere saturation, adaptive non-additive overlap correction, V0/V1/V2, integrator and diagnostics are implemented | One Linux x86_64 run on rustc 1.88.0: `cargo test --locked --workspace --all-targets` gave 142 passed, 0 failed, 4 ignored. Log `/tmp/sg-ws7c-evidence/test.txt`, commit `0a443e2` (docs and scripts on top of `52058f4`). Not a Windows or macOS result. |
| Test (c) certified screening | A certified quadrature preset and a measured result are documented in `CLAIMS.md` / `shadowgrav-core/CONVERGENCE.md` | Repository-reported result, not re-run in this review |
| Native UI | egui desktop instrument with 3D view, force arrows, sky diagnostic, presets, conformance status and snapshot export | Source and headless UI tests present; native window still needs a real desktop-session proof |
| Multi-OS CI | Core and native UI workflows configured for Ubuntu, macOS and Windows; reproducibility digest comparison configured | Configuration is present; no live GitHub Actions run/result was supplied with this source archive |
| Export | Rectangular 27-column CSV plus same-stem JSON metadata, column dictionary and code-unit map | Schema has a UI unit test; core tests run successfully in this environment |

**New:** run scenarios from JSON with `sgrun run` or from Python (`python/`); see **TUTORIAL.md**.

Read **CLAIMS.md** before quoting model results. Read **SCIENTIFIC_MANUAL.md** for operating guidance and **SCOPE_v1.md** for declared limits. The project should not be described as a fully certified research platform until the remaining traceability gaps and external CI evidence have been closed.

---

## Additional analysis and comparison tools (2026-11-07)
- `ERROR_BUDGETS.md` — published oracle error budgets for unequal-radius geometries.

- `REPRODUCIBILITY.md` — digests and multi-OS comparison template.



- `MODEL_VS_OBSERVATION.md` — explicit table of what the model does and does not reproduce.

- `python/analysis/analyze_export.py` — plot and summarize any trajectory or snapshot CSV without reading Rust.

- `oracle/oracle_unequal_radius.py` — independent grid oracle for unequal radii.

- `IMPROVEMENTS_2026-11-07.md` — record of the product-quality pass performed in this environment.


## Quick start

### Requirements

- Rust. `rust-toolchain.toml` pins 1.88.0, the tested workspace and UI minimum. rustc 1.87.0 rejects locked `icu_*` 2.4.0 and `image` 0.25.10. The UI uses eframe 0.32.0 (egui 0.32.3). `uuid` is locked at 1.26.1 because 1.27+ needs rustc 1.89 and zbus only requires `^1.8`. rustc 1.83.0 cannot parse locked `calloop` 0.14.5 (`edition2024`). Core built outside the workspace (`scripts/core-isolated-msrv.sh`, `shadowgrav-core/Cargo.lock`, rayon 1.12.0) minimum is 1.80.0. `--no-default-features` checks on 1.65.0 and fails on 1.64.0 (`let...else`).
- Windows / macOS / Linux desktop for the native UI

### Build and run

```bash
cargo build --release --locked -p shadowgrav-ui
cargo run --release --locked -p shadowgrav-ui
```

### Verify

This is the section `CLAIMS.md` points at. The words "in 10 minutes" are not a measured duration.

### Automated checks

```bash
# Workspace tests, including core, headless UI tests, and example harnesses.
# Recorded on rustc 1.88.0 (Linux x86_64): 142 passed, 0 failed, 4 ignored.
# Workspace test profile is opt-level 3. This invocation is not --release.
cargo test --locked --workspace --all-targets

# Separate invocation. No count from this command is recorded in this file.
cargo test --release --locked --workspace

# Screening measurement ladder
cargo run --release --locked -p shadowgrav-core --bin verify -- c

# Certified Spec test (c) screening gate
cargo run --release --locked -p shadowgrav-core --bin sgrun -- certify
```

The certified screening command is a targeted check, not a substitute for the complete suite or for the other conformance tests (a), (b), (d), and (e).

### Capture machine proof

Linux / macOS:

```bash
./scripts/local-proof.sh --launch-ui
```

Windows PowerShell:

```powershell
.\scripts\local-proof.ps1 -LaunchUI
```

These scripts capture OS/toolchain details, workspace test logs, UI build logs and (when requested) the native application launch. Follow `MACHINE_PROOF.md` to capture visual evidence; successful compilation alone is not a GUI proof.

### Interactive checks

1. Preset **(a)** — inspect the orbit and conservation diagnostics.
2. Preset **(c)** — inspect R, |R−0.8|, overlap correction and quadrature tier; the Certified tier is expensive.
3. Enable individual force arrows and compare common-scale versus per-component scaling.
4. Inspect the selected body's sky map; remember it is diagnostic only and is not used to calculate forces.
5. Press **E** — export a rectangular CSV and a same-stem `.meta.json` sidecar.
6. Enable **thin-fast** only as an explicitly labelled non-conforming preview.

---

## Repository map

```text
shadowgrav-core/   force engine, integrator, diagnostics, conformance tests
shadowgrav-ui/     native egui instrument
oracle/            independent Python oracle/reference scripts
report/            convergence data and figures
scripts/           machine-proof capture scripts
CLAIMS.md          permitted claims and explicit non-claims
SCIENTIFIC_MANUAL.md
SCOPE_v1.md
MACHINE_PROOF.md   local desktop and CI evidence checklist
AUDIT_2026-11-06.md
```

## Quadrature tiers

| Tier | Intended use |
|---|---|
| Standard | Exploration and rapid iteration |
| Validation | Dynamics-grade interactive work |
| Fine | Higher-resolution interactive checks; still short of the Spec's 1e-6 test (c) target |
| Certified | Targeted Spec test (c) screening gate; expensive and geometry-specific |

## Release discipline

A release candidate needs the command list in `RELEASING.md`, including a fresh `cargo test --locked --workspace --all-targets` log, a real native-window run on the target machine, and green Ubuntu/macOS/Windows results from a run of `.github/workflows/ws4-matrix.yml`. That workflow has not been run. Keep the test logs, run metadata, screenshots and commit hash together. Do not upgrade a source-level claim into an executed result without its corresponding evidence.
