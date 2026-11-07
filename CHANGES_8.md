# workspace_8: changes since workspace_7
1. UI: fixed E0503 in shadowgrav-ui/src/app.rs (thin-fast checkbox colour read into a local before the mutable borrow). NOT compiled here (no egui); the owner's compiler reported this as the blocking error.
2. Trig routing: healpix.rs (sin_cos), gl.rs (cos), driver.rs (cbrt, atan2, ln) now go through mathx; new mathx::cos, mathx::ln. Default build: sgrun repro-hash identical before/after (f2e108670c118a3d). vendored-libm build not compiled here.
3. Reference mode (Spec 4.4): src/reference.rs + tests/reference_mode.rs. It is the all-rays evaluation of the SHADOW model (eq. 3), not a Newtonian sum (a Newtonian loop is a different force law and could validate nothing). Scope: V0, alpha_bg = 0, bodies only, exterior pairs, N <= 50; anything else returns an error. Returns the force plus its own resolution estimate. reference_check(world) gives the engine-vs-reference error (Spec 6 item 11); not yet shown in the UI.
Measured (relative to the largest force): engine validation vs reference 1.7e-6..2.8e-6 (partial/3D overlap), standard 1.8e-6..1.2e-4. (Per-body-relative standard error on the weakest body reaches 9.6e-4: both metrics are valid, state which.)
Tests: core suite all green in the sandbox (serial build); UI headless 6/6.

## Added after the first delivery (no owner action needed)
- tests/edge_cases.rs (3 tests, pass): cap-hit flag fires with ell_cap = 3 and not with the default (D4); NaN/Inf positions rejected (A7, construction only); a coincident body adds exactly 0 to the pair force (A8).
- .github/workflows/ci.yml: core on Linux/macOS/Windows, serial+vendored-libm, UI build + headless tests, cross-OS repro-hash comparison, manual `sgrun certify`. Never run: the first push will show whether action versions and apt packages are right.
- TRACEABILITY: D4, A8 -> PASS; totals 39 PASS / 17 PARTIAL / 4 GAP.
