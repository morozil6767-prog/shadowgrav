# shadowgrav workspace quick reference

```bash
# Full workspace tests (core + headless native-UI/protocol tests)
cargo test --release --locked --workspace

# Build/run the desktop instrument
cargo build --release --locked -p shadowgrav-ui
cargo run --release --locked -p shadowgrav-ui

# Screening measurement and certified test (c) gate
cargo run --release --locked -p shadowgrav-core --bin verify -- c
cargo run --release --locked -p shadowgrav-core --bin sgrun -- certify

# Candidate graph / parallel equivalence / preview gating
cargo test --release --locked -p shadowgrav-core --test perf_equiv
```

`shadowgrav-ui/DESIGN.md` documents the native UI architecture and limitations. `shadowgrav-core/TRACEABILITY.md`
tracks every frozen-spec requirement against tests, asserted tolerances and evidence tags; read it before calling the
model conforming. `MACHINE_PROOF.md` explains how to capture local OS, build, test, native-window and multi-OS CI evidence.

The certified screening result documented in `CLAIMS.md` is a targeted result for the prescribed collinear test (c), not
a blanket proof of all model regimes. Multi-OS workflow configuration lives in `.github/workflows/ci.yml`; green Actions
results must be collected from the actual repository after push.
