# Verification protocol for shadowgrav-core

This guide is for the current workspace tree. It separates source implementation, automated tests, previously reported
measurements, and newly captured evidence. See `TRACEABILITY.md` for the full frozen-spec requirement ledger.

## 1. Reproduce the checkout

From the workspace root:

```bash
rustc --version --verbose
cargo --version
cargo test --release --locked --workspace
```

The workspace test command runs the core test binaries and the headless UI/protocol tests. If it fails, retain the
complete output and toolchain/OS details; do not infer that a mathematical requirement passed because a related test
compiled.

Targeted suites useful when triaging changes:

```bash
cargo test --release --locked -p shadowgrav-core --test hardening
cargo test --release --locked -p shadowgrav-core --test geometry_suite
cargo test --release --locked -p shadowgrav-core --test oracle_lens
cargo test --release --locked -p shadowgrav-core --test oracle_sky
cargo test --release --locked -p shadowgrav-core --test perf_equiv
cargo test --release --locked -p shadowgrav-ui
```

## 2. Prescribed measurements

```bash
# Screening curve / quadrature ladder
cargo run --release --locked -p shadowgrav-core --bin verify -- c

# Certified quadrature gate for the prescribed three-sphere test (c)
cargo run --release --locked -p shadowgrav-core --bin sgrun -- certify
```

The source tree documents the following earlier measurements in `ADDENDUM3.md` and `CONVERGENCE.md` (evidence tag S,
reported from a prior serial Rust workspace run): Standard had about 1.29e-3 error; Validation about 1.58e-5; Fine/xhi
about 3.32e-6; and `Validation::certified()` about 4.31e-7 at tau_c=20 using roughly 64.8 million leaves with zero cap
hits. Only the Certified preset met the specified 1e-6 target on that prescribed geometry. These are repository-reported
measurements, not execution results from the 2026-11-06 source audit, and they do not establish 1e-6 accuracy for arbitrary
multi-body geometries.

## 3. Geometry and numerical regression

The current tree includes `geometry_suite.rs` for collinear screening, partial and 3D overlap layouts, momentum residuals,
translation/rotation equivariance of the exact two-body path, and a collimated multi-occluder stress layout. `hardening.rs`
checks malformed structure-of-arrays inputs, inconsistent derived buffers, non-finite state, global parameters, derived
physical ranges, heating controls and invalid mutations of publicly mutable `World` state.

The rest of the conformance surface is in `identities.rs`, `integrator.rs`, `edge_cases.rs`, `healpix_ref.rs`,
`oracle_lens.rs`, `oracle_sky.rs`, `reference_mode.rs`, `spec_gaps.rs`, `diag_gaps.rs` and `perf_equiv.rs`. For each
requirement, use the asserted tolerance and evidence tag in `TRACEABILITY.md`; do not substitute an easier smoke test for
the exact contract test.

## 4. Numerical contract differences that need explicit governance

The uploaded frozen mathematical specification and the repository's proposed addenda are not textually identical. In
particular, `ADDENDUM2.md` proposes tightening the adaptive `h*gamma` cap from 0.1 to 0.01 and changing the fixed-point
residual denominator to `|v| + (h/2)|a| + 1e-30`. The current `integrator.rs` implements those changes. The frozen PDF's
eq. 12 specifies `|v^(i+1)| + 1e-30`, and eq. 13 specifies the 0.1 cap. Treat the addendum as a proposed contract change
unless it has been formally adopted by the model owner.

Similarly, `derive.rs` evaluates the Taylor series for A(tau) up to tau<1 (the PDF says tau<0.05); the code rationale is
cancellation avoidance, but its finite 40-term series and tests should remain in the numerical error budget. Keep that
choice visible in the traceability discussion.

## 5. Multi-OS and native UI evidence

The GitHub Actions workflow now tests core and builds/tests the UI on Ubuntu, macOS and Windows, runs serial vendored-libm
tests, compares per-platform reproducibility digests, and schedules the certified test (c). A workflow definition is not
a successful run. The actual logs and artifacts must be collected from GitHub Actions for the commit under review.

For a real local machine run, use `scripts/local-proof.sh --launch-ui` or `scripts/local-proof.ps1 -LaunchUI` and follow
`MACHINE_PROOF.md`. A successful build or headless UI test is not a native-window visual check; capture screenshots of
the actual app and the 3D/sky/export behaviours listed there.

## 6. Conformance statement

Do not call the implementation conforming merely because `cargo test` is green. The frozen PDF requires tests (a)–(e),
all diagnostics in section 6, the exact pair and overlap force law, saturation, finite spheres, deficit-form transport and
calibration. Use `TRACEABILITY.md` to identify any remaining partials/gaps, and only state a pass when the current checkout
has a test result with the appropriate tolerance and evidence tag.
