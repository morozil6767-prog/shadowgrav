# Machine and CI proof protocol

This project is intended to be used as a research instrument. Source inspection, a passing headless test, and a screenshot
are different kinds of evidence; keep them separate and retain the commit hash with all artifacts.

## 1. Local machine proof

Run from the repository root on the actual target machine.

**Linux / macOS**

```bash
./scripts/local-proof.sh --launch-ui
```

**Windows PowerShell**

```powershell
.\scripts\local-proof.ps1 -LaunchUI
```

The scripts create a timestamped `proof/local-*` directory containing OS/toolchain information, workspace test output,
UI build output, launch output (when requested), and exit codes. They do not manufacture a screenshot or claim visual
success automatically.

## 2. Native-window checklist

With the actual desktop window open, capture screenshots and note the machine/display configuration for:

1. **Startup and layout:** full window at the default 1500×900 size; panels, conformance badge, status strip and sky panel
   visible without overlap or clipped text.
2. **3D geometry:** presets (a), (c), and a 3D partial-overlap case; rotate, zoom, fit, axis views and body picking.
3. **Force arrows:** total/pair/correction/drag/aberration toggles; selected-body vs all-body; common vs per-component scale;
   verify the legend identifies the scale and line-of-sight markers remain visible.
4. **Sky diagnostic:** deficit and non-additive modes; all-sky and disc zoom; labels and force directions; record that this
   map is diagnostic-only and is not the force quadrature.
5. **Conformance signalling:** show Green, Amber and Red states where available; include a thin-fast run and verify the
   non-conforming label is impossible to miss.
6. **Export:** press E; check that the `.csv` is rectangular with one header and one row per body/probe, and that the
   same-stem `.meta.json` contains parameters, run state, diagnostic summary and conformance reasons.
7. **Responsiveness:** test a representative many-body case; record particle count, selected quadrature tier, steps/s,
   sky-map state and machine specs. Do not infer GPU performance—the current view is CPU/egui.

Store unedited screenshots in the proof directory. A screenshot should show the app itself, not a mock-up or code editor.
Record any failed checklist item rather than omitting it.

## 3. Actual multi-OS CI proof

After pushing the commit to GitHub, open the `ci` workflow run and retain:

- Core test logs for Ubuntu, macOS and Windows.
- Native UI build and test logs for Ubuntu, macOS and Windows.
- Serial vendored-libm test log.
- Per-OS `sgrun repro-hash` artifacts and the digest-comparison result.
- Certified test (c) output from the scheduled or manually dispatched certification job.

A workflow file is only a recipe. The phrase “multi-OS CI passed” is justified only by a completed green run for the exact
commit. If a runner fails, preserve its log and report the platform-specific failure.

## 4. Evidence labels for papers and release notes

Use these labels consistently:

- **Source implemented:** code path exists.
- **Test authored:** test exists but has not been executed on the stated environment.
- **Locally executed:** test/build log captured on a named machine/toolchain.
- **CI executed:** GitHub Actions log captured for the exact commit and OS.
- **Visually verified:** native app was opened and the corresponding screenshot was captured.
- **Conformance demonstrated:** all applicable frozen-spec tests and diagnostics pass at their stated tolerances, with the
  correct regime/limitations reported.

Do not promote one evidence class into another.

## 4. Sandbox / CI-only environments

A headless Linux sandbox (or pure CI runner) cannot produce a native-window screenshot. In such environments:

- Record the `sgrun repro-hash` combined digest and individual hashes.
- Run `cargo test --release --locked -p shadowgrav-core` and retain the log.
- Explicitly state that native-window visual proof is pending a real desktop session.
- Do not claim UI visual correctness from headless tests alone.

### Template for a machine-proof report

```
Machine: <OS> <arch> <cpu>
Rust: <rustc --version>
Commit: <git rev-parse HEAD>
Date: <ISO>

Core tests: PASS / FAIL (log attached)
sgrun repro-hash combined: <hex>
Native window: PENDING / PASS (screenshots attached) / FAIL (notes)
Notes: <any deviations>
```

Attach the raw log files; do not summarise away failures.
