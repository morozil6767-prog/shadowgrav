# workspace_6: changes since workspace_5
- Test (c): the claimed xhi result 6.96e-7 did NOT reproduce (xhi = 3.32e-6). New Validation::certified() (f_v 7e-4, cap 15, ~65M leaves, ~14 s) gives 4.3e-7 at tau 20 and <= 4.3e-7 for tau 0.1..50. Lock test replaced (tests/integrator.rs).
- Oracle-test tolerances now come from measurements (tests/oracle_sky.rs). Standard mode is coarse on partial overlap: Delta^NA up to 8.6% off, force up to 9.6e-4.
- New: bin sgrun (certify, scan, orbit, drag, skymap, repro-hash; JSON provenance), bin converge, build.rs (git/rustc stamp), diagnostics::eta_ep, tests/spec_gaps.rs, skymap overlap-support test, presets (c2)/(c3) with oracle read-outs, oracle/oracle_screening_scipy.py + screening_curve.csv, report/ (data + make_figures.py + 3 figures).
- UI text corrected (sim.rs, app.rs); README_WORKSPACE and ADDENDUM2 corrected.
- NOT done: ADDENDUM3.md, CONVERGENCE.md, TRACEABILITY.md, technical report text, CI workflow, final full-suite rerun. egui files still uncompiled (no crates.io here).
