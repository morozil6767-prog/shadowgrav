#!/usr/bin/env bash
# Rebuild the local evidence bundle on rustc 1.88.0.
# Output stays outside the repo unless you pass a directory under it.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
stamp=$(date -u +%Y%m%dT%H%M%SZ)
out=${1:-"/tmp/sg-evidence-${stamp}"}
mkdir -p "$out"
cd "$root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/sg-evidence-target}"
{
  echo "commit=$(git rev-parse HEAD)"
  echo "branch=$(git rev-parse --abbrev-ref HEAD)"
  echo "utc=${stamp}"
  rustup run 1.88.0 rustc -V
  rustup run 1.88.0 cargo -V
} | tee "$out/version.txt"
set +e
rustup run 1.88.0 cargo fmt --all --check >"$out/fmt.txt" 2>&1
echo $? >"$out/fmt.exit"
rustup run 1.88.0 cargo clippy --locked --workspace --all-targets -- -D warnings >"$out/clippy.txt" 2>&1
echo $? >"$out/clippy.exit"
rustup run 1.88.0 cargo test --locked --workspace --all-targets >"$out/test.txt" 2>&1
echo $? >"$out/test.exit"
rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin sgrun -- repro-hash >"$out/repro-hash.txt" 2>&1
echo $? >"$out/repro-hash.exit"
rustup run 1.88.0 cargo run --release --locked -p shadowgrav-core --bin verify -- spec-a >"$out/verify-spec-a.txt" 2>&1
echo $? >"$out/verify-spec-a.exit"
set -e
(
  cd "$out"
  sha256sum version.txt fmt.txt fmt.exit clippy.txt clippy.exit test.txt test.exit \
    repro-hash.txt repro-hash.exit verify-spec-a.txt verify-spec-a.exit > SHA256SUMS
)
echo "evidence_dir=$out"
