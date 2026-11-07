# Fixes applied after independent verification (Claude, sandbox, Rust 1.97, core built dependency-free)

Only `shadowgrav-core/tests/diag_gaps.rs` changed. No engine source changed; `sgrun repro-hash` stays f2e108670c118a3d.

- midrun_nan_velocity...: a NaN velocity is rejected by World::validate (InvalidParticle(1)) before the integrator; test now asserts that and that state is untouched.
- midrun_non_finite_force... and non_finite_force_from_a_live_edit...: Pi=Inf is rejected as invalid config. Overflow is now produced with finite valid values (pi_override=1e300, xi=1e300 -> Gamma_k overflows), reaching the real CoreError::NonFinite path and its rollback.

Result: core 109 tests, all pass (S). Headless UI modules (protocol/sim/presets/export) 7 tests pass (S).
Not executed anywhere by me: egui UI compile, CI workflow, vendored-libm build, multi-OS hash, rayon path.
