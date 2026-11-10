#!/usr/bin/env bash
# Test shadowgrav-core outside the workspace.
# The workspace Cargo.lock contains edition-2024 crates, so cargo older than 1.85
# cannot parse it. This copy uses shadowgrav-core/Cargo.lock (rayon 1.12.0).
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cp -a "$root/shadowgrav-core/." "$tmp/"
cd "$tmp"
echo "isolated core dir: $tmp"
rustc --version --verbose
cargo --version
cargo test --release --locked --all-targets "$@"
