# shadowgrav-core

Scalar, IEEE-754 binary64 **reference engine** for a classical, non-relativistic, *non-additive*
shadow-gravity ("Le Sage-type") N-body model, together with an integrator, diagnostics and a
verification harness.

> **Status (v1.0-candidate):** Core test suites pass on release builds (identities, integrator, oracles,
> reference mode, edge cases, diag_gaps). Official claims and limits: see repo root `CLAIMS.md` and
> `SCIENTIFIC_MANUAL.md`. Re-run `cargo test --release -p shadowgrav-core` on your machine before citing.

## 1. Scientific goal

The simulation tests the *shadow mechanism*, nothing more: bodies are uniform finite spheres that
attenuate an isotropic momentum flux; the net force on a body is the exact attenuated-ray integral
(Spec eq. 3/4), including saturation (S != 1), overlap ("non-additive") screening, optional background
extinction (Seeliger-type, alpha_bg), ambient drag (V1) and first-order aberration (V2).
It does **not** test Sakharov's induced gravity or Puthoff's ZPF model, and it must not be used to
claim GR phenomena, cosmology or Lorentz covariance (Spec section 8).

## 2. Implementation status and assurance

The following table describes what is present in this source tree. Presence of a code path or a test is not itself proof that the current checkout has been compiled and executed on every target platform.

| Area | Implementation in this tree | Assurance notes |
|---|---|---|
| Derived quantities and calibration | `derive.rs`, `types.rs` implement eqs. 1–2 and 19 | `identities.rs`, `spec_gaps.rs`, and `hardening.rs` contain related checks |
| Pair force and finite-size saturation | `pair.rs`, `deficit.rs`, `geom.rs` implement pair-exact exterior/interior paths and ray chords | `identities.rs`, `edge_cases.rs` exercise thin, saturated and contact regimes |
| Non-additive overlap | `overlap.rs`, `healpix.rs`, `force.rs` implement the adaptive correction and candidate graph | `oracle_lens.rs`, `oracle_sky.rs`, `geometry_suite.rs`, `perf_equiv.rs`; the most demanding oracle and parallel checks still require confirmation on the intended release runners |
| V0/V1/V2 | `velocity.rs`, `pair.rs`, `overlap.rs`, `force.rs` | V2 remains restricted by the spec and has first-order-reference/oracle limitations documented in `TRACEABILITY.md` |
| Integrator and step controls | `integrator.rs`, `state.rs` implement Verlet, fixed-point and adaptive-step paths | `integrator.rs` tests include reversibility, drag, frame invariance and adaptive-step checks |
| Diagnostics and reference mode | `diagnostics.rs`, `reference.rs`, `driver.rs` | Some diagnostic cross-checks remain partial/gaps in `TRACEABILITY.md`; do not treat every displayed value as independently oracle-certified |
| Native UI / sky / arrows | Implemented in `shadowgrav-ui` | UI source and headless protocol/simulation tests are present; visual confirmation must be done in a real desktop session |
| Cross-OS reproducibility | GitHub Actions matrix in `.github/workflows/ci.yml` | Workflow configuration is not a result; collect green Ubuntu, macOS and Windows run evidence |

### Public API validation

The current source validates particle structure-of-arrays lengths and finite state, physical body inputs, global calibration/quadrature parameters, options, and derived-array shape/physical ranges at the `derive` and force-evaluation boundaries. `SimConfig` also validates the energy-deposition parameters. Negative tests for these cases are in `tests/hardening.rs`.

### Known assurance gaps

Use `TRACEABILITY.md` as the detailed specification-to-test ledger. It marks the remaining partials/gaps explicitly, including several diagnostic cross-checks, some background-absorption momentum checks, V2 scope/accuracy, multi-thread execution evidence and native egui visual evidence. The reported certified test (c) measurement is a targeted geometry result, not blanket proof of the complete model over all configurations.

## 3. Build

Requires Rust >= 1.70 for the core crate (the native UI crate declares Rust >= 1.76). The first build needs access to the
crates.io index (Cargo resolves the optional `libm` dependency even when the feature is off).

```
cargo build --release
cargo build --release --features vendored-libm     # optional: platform-independent libm
```

## 4. Verification sequence

From the workspace root:

```bash
cargo test --release --locked --workspace
cargo test --release --locked -p shadowgrav-core --test hardening
cargo test --release --locked -p shadowgrav-core --test geometry_suite
cargo run --release --locked -p shadowgrav-core --bin verify -- c
cargo run --release --locked -p shadowgrav-core --bin sgrun -- certify
```

The full workspace command includes both the core tests and the headless UI/protocol tests. The extra commands isolate
new public-API validation, geometry regression and the expensive screening benchmark. `sgrun certify` is targeted at
Spec test (c); it does not replace the full suite or certify every possible geometry and parameter regime.

## 5. Running the test (c) measurement

```
cargo run --release --bin verify -- c
```

It evaluates three collinear spheres (d_AB = d_BC = 10a, tau_c = 20) and prints, for standard mode at
f = 0.15 / 0.075 / 0.0375 and for validation mode (f_v = 0.02, f_s = 4e-4, cap 14): the ratio
R = |F_A|/|F_AB + F_AC|, |R - 0.8|, F_B/F_A, |F_A + F_C|/|F_A|, leaf count, cap-hit count and wall time;
then R(tau) for tau from 1e-3 to 50 (should decrease monotonically from 1). A per-target leaf budget of
1e8 aborts runaway cases explicitly ("ABORTED") rather than coarsening silently.

## 6. Known limitations and risks

1. The model is classical, static/non-relativistic ray transport with geometric optics. It does not model diffraction, finite wavelength, GR effects, Lorentz covariance, cosmology, or a derived microscopic replenishment mechanism.
2. Target attenuation is center-sampled. Near-contact and overlap configurations can have O(a/d) center-sampling error; overlapping spheres of different radii are not exactly reciprocal.
3. Scattered flux is removed rather than re-tracked. The background relaxation law is phenomenological.
4. V1 drag is unshielded. V2 is limited to alpha_bg = 0 and max |eps| <= 0.1; shifted-disc geometry and the first-order formula have explicit scope/accuracy caveats.
5. The overlap correction uses adaptive HEALPix midpoint quadrature. Silhouette-edge error and refinement-cap hits must be monitored; cap-hit configurations are not trustworthy.
6. Long-time energy conservation is expected only in the specified reciprocal/conservative regime. Elsewhere energy is a diagnostic and may drift physically.
7. The high-accuracy screening preset is expensive and its documented error applies to the benchmark geometry, not arbitrary overlap layouts.
8. `thin-fast` is a non-conforming Barnes–Hut preview path and must remain visibly labelled as such; it is not evidence for the full force law.
9. Cross-platform bitwise identity, native UI correctness and release readiness require actual runner logs and desktop-session evidence. Source configuration alone is insufficient.

Deliberate implementation choices and the full per-requirement evidence ledger remain documented in `TRACEABILITY.md` and the addenda.

## 7. Mapping to the Specification and Addendum

| Spec item | Code |
|---|---|
| eq. 1-2 (a, kappa, beta, tau_c, A(tau), A_eff, S, m^g) | `derive.rs` (`derive`, `a_fun`) |
| eq. 3-4 assembly, probes, edge cases 1.6 | `force.rs` (`evaluate_forces`) |
| eq. 5 single-occluder deficit, eq. 17 sweep | `deficit.rs` |
| eq. 16 chord | `geom.rs` (`chord`) |
| eq. 6a / 6b pair integrals | `pair.rs` (`exterior_nodes_phi`, `interior_nodes`, `pair_p`) |
| eq. 20 moments (Omega, tau_bar) | `pair.rs` (`exterior_nodes_sinh`, `pair_moments`) |
| eq. 7, 19 calibration | `types.rs` (`Globals::pi_flux`) |
| eq. 8 drag | `velocity.rs` |
| eq. 9, Addendum I.2-I.4 (aberration, shifted discs) | `pair.rs` (`pair_e`), `overlap.rs` (`corr_shifted`, two touching sets) |
| section 4.3 adaptive HEALPix correction | `overlap.rs`, `healpix.rs` |
| Addendum II.4 validation mode | `Options::validation`, `overlap.rs::cell_body` |
| eq. 11-13 integration | `integrator.rs` |
| section 3.4 conservation, section 6 diagnostics | `diagnostics.rs` |
| section 7 tests (a), (d1), (d2) | `driver.rs`, `tests/integrator.rs` |
| section 7 test (b), thin limit, saturation, reciprocity | `tests/identities.rs` |
| section 7 test (c) | `tests/identities.rs::screening_standard_mode_provisional` (5e-3 only), `verify -- c` |
| section 7 test (d3) | `tests/identities.rs::v2_matches_eq10_thin_pair`, `driver::run_v2_vs_v1` |
| section 7 test (e) | heating: `tests/integrator.rs`; Seeliger single pair: `tests/identities.rs`; Seeliger paired read-out: `tests/diag_gaps.rs` (unexecuted); eta_EP: `tests/spec_gaps.rs` |

## 8. Layout

```text
shadowgrav-core/  src/ force/integrator/diagnostics modules; tests/ conformance suites
shadowgrav-ui/    src/ app, simulation protocol, 3D view, sky view and export
oracle/           independent Python oracle scripts and comparison data
report/           convergence data and figures
scripts/          local machine-proof capture
```

Layering: `driver -> diagnostics -> integrator -> state -> force core`. The force core is a pure function
`evaluate_forces(&Particles, &Derived, &Globals, &Options)`; the UI exchanges immutable snapshots and commands with the
simulation thread rather than reaching into `World`.
