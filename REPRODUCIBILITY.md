# Cross-platform reproducibility

`sgrun repro-hash` produces a stable FNV-1a digest of canonical force and state vectors on a given OS/architecture after trigonometric functions are routed through `mathx`. Cross-OS bitwise identity is not guaranteed without the `vendored-libm` feature and verified CI runs.

## Recorded digest (Linux x86_64, rustc 1.99.1, 2026-11-07)

| Case | Digest |
|------|--------|
| orbit_forces | e60572a3d079862d |
| orbit_state_200 | c643d780b40120fd |
| screen_std | 9eee9748d2156d26 |
| screen_val | 5dd3fd3f4ebe8c39 |
| overlap3d_std | 2de33d701849ff93 |
| orbit_v1 | 3b51b87fbd2ee11d |
| orbit_v2 | 35e7dd4558237473 |
| **combined** | **f2e108670c118a3d** |

Provenance: parallel feature on, vendored_libm off, 2 threads.

## How to collect a digest on another OS

```bash
cargo build --release --locked -p shadowgrav-core --bin sgrun
./target/release/sgrun repro-hash
```

Copy the full JSON line and the individual hashes into a release note or this file under a new dated heading.

## Comparison template for a release

| OS / arch | rustc | combined digest | matches Linux reference? | notes |
|-----------|-------|-----------------|--------------------------|-------|
| Linux x86_64 | 1.99.1 | f2e108670c118a3d | reference | this environment |
| macOS arm64 | | | | |
| Windows x86_64 | | | | |

A green multi-OS CI run should attach the workflow logs that contain these digests. Until those logs exist, cross-OS bitwise identity remains an open claim (see CLAIMS.md).

## Residual risk

Without `vendored-libm`, platform libm differences can change the least-significant bits of trigonometric evaluations. The current routing through `mathx` reduces but does not eliminate this. Enable the feature and re-hash if bitwise identity across OSes is required.
