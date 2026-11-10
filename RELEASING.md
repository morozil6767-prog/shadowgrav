# Releasing

Do not tag while a blocker below is open. This file does not add a `LICENSE` file and does not add a `license` field to any `Cargo.toml`.

## Blockers

1. Licence not chosen.
2. `deny.toml` sets `unmaintained = "all"` and `ignore = []`. That policy is an owner decision. This checklist does not edit `deny.toml`. `cargo deny` was not re-run while writing this file.
3. `.github/workflows/ws4-matrix.yml` has not been run. The three-OS digest table below is empty on purpose. A Linux digest is not a substitute for the macOS or Windows rows.

`Validation::graded` is not the certified tier. Test (a) stays at the `5e-6` bound. This checklist does not change either.

## Versions now

| crate | `Cargo.toml` version |
|---|---|
| `shadowgrav-core` | `0.2.1` |
| `shadowgrav-ui` | `0.1.0` |

Bump both in the same commit as the changelog entry for the release. Do not tag a dirty tree.

## Commands

Run from a clean checkout. Prefix each `cargo` line with `rustup run 1.88.0` so the toolchain is the workspace pin, not whichever rustc is first on `PATH`.

```bash
rustup run 1.88.0 rustc -V
rustup run 1.88.0 cargo fmt --all --check
rustup run 1.88.0 cargo clippy --locked --workspace --all-targets -- -D warnings
rustup run 1.88.0 cargo test --locked --workspace --all-targets
rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin sgrun -- repro-hash
rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin verify -- spec-a
```

Core minimum, separate tree, rustc 1.80.0:

```bash
rustup run 1.80.0 ./scripts/core-isolated-msrv.sh
```

That script was not run while writing this file.

One recorded workspace run, not a release run of this branch: rustc `1.88.0 (6b00bc388 2025-06-23)`, `cargo test --locked --workspace --all-targets`, 24 harnesses, 142 passed, 0 failed, 4 ignored. Log `/tmp/sg-ws7c-evidence/test.txt`. Commit of that run `0a443e2e2eb289f1980feb2ad44c735549ea61df` (docs and `scripts/make-evidence.sh` on top of `52058f4`). Combined repro-hash in `/tmp/sg-ws7c-evidence/repro-hash.txt`: `f2e108670c118a3d`. `verify spec-a` in that directory exited 0. Those files are outside the repository.

`scripts/make-evidence.sh` is on branch `cursor/ws7-evidence-a7be`. It is not on this branch. Until that script is in the tree being released, keep the logs of the commands above and hash them with `sha256sum`. Keep that directory outside the repo.

## Three-OS repro-hash

`ws4-matrix.yml` is `workflow_dispatch` only. It was not dispatched. Fill the table from that run's logs. Leave a cell blank if that OS did not finish.

| OS | combined digest | log |
|---|---|---|
| ubuntu-latest | not run | |
| macos-latest | not run | |
| windows-latest | not run | |

The Linux line `f2e108670c118a3d` above is one machine. It does not fill this table.

## Tag

After the blockers are closed and the three rows are filled from logs, tag the release commit. This file does not create a tag.
