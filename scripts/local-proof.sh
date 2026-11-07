#!/usr/bin/env bash
# Capture machine-specific build/test evidence. Run from any directory.
set -u -o pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="${SHADOWGRAV_PROOF_DIR:-$ROOT/proof/local-$STAMP}"
mkdir -p "$OUT"
cd "$ROOT"

capture_machine() {
  local rc=0
  echo "timestamp_utc=$STAMP"
  echo "hostname=$(hostname 2>/dev/null || echo unavailable)"
  echo "kernel=$(uname -a 2>/dev/null || echo unavailable)"
  echo "architecture=$(uname -m 2>/dev/null || echo unavailable)"
  echo "os_release=$(cat /etc/os-release 2>/dev/null | tr '\n' ';' || sw_vers 2>/dev/null | tr '\n' ';' || echo unavailable)"
  echo "display=${DISPLAY:-unset}"
  echo "wayland_display=${WAYLAND_DISPLAY:-unset}"
  echo "gpu_tools=$(command -v glxinfo || command -v vulkaninfo || echo not-installed)"
  echo
  echo "--- rustc ---"
  rustc --version --verbose 2>&1 || rc=1
  echo
  echo "--- cargo ---"
  cargo --version 2>&1 || rc=1
  return "$rc"
}
set +e
capture_machine 2>&1 | tee "$OUT/machine.txt"
machine_code=${PIPESTATUS[0]}

set +e
cargo test --release --locked --workspace 2>&1 | tee "$OUT/workspace-tests.txt"
test_code=${PIPESTATUS[0]}
cargo build --release --locked -p shadowgrav-ui 2>&1 | tee "$OUT/ui-build.txt"
cargo run --release --locked -p shadowgrav-core --bin sgrun -- repro-hash 2>&1 | tee "$OUT/repro-hash.txt" || true
build_code=${PIPESTATUS[0]}
set -e
printf 'machine_info_exit=%s\nworkspace_test_exit=%s\nui_build_exit=%s\n' \
  "$machine_code" "$test_code" "$build_code" | tee "$OUT/result.txt"

launch_code=0
if [[ "${1:-}" == "--launch-ui" && "$test_code" -eq 0 && "$build_code" -eq 0 ]]; then
  echo "Launching the native window. Close it normally after checking the proof checklist." | tee "$OUT/ui-launch.txt"
  set +e
  cargo run --release --locked -p shadowgrav-ui 2>&1 | tee -a "$OUT/ui-launch.txt"
  launch_code=${PIPESTATUS[0]}
  set -e
  printf 'ui_launch_exit=%s\n' "$launch_code" | tee -a "$OUT/result.txt"
fi

cat <<EOF | tee "$OUT/README.txt"
Shadowgrav local machine proof
==============================
Evidence directory: $OUT

Captured: machine.txt, workspace-tests.txt, ui-build.txt, result.txt

For native-window evidence, run this script with --launch-ui on a desktop session.
While the window is open, verify the checklist in MACHINE_PROOF.md and save screenshots
into this directory. A successful compile alone is not a successful UI proof.

Exit codes: machine info=$machine_code, workspace tests=$test_code, UI build=$build_code, UI launch=$launch_code
EOF

[[ "$machine_code" -eq 0 && "$test_code" -eq 0 && "$build_code" -eq 0 && "$launch_code" -eq 0 ]]
