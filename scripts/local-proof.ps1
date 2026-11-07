# Capture machine-specific build/test evidence on Windows PowerShell 5.1+ or PowerShell 7+.
param([switch]$LaunchUI)
$ErrorActionPreference = 'Continue'
$LaunchCode = 0
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Stamp = [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')
$ProofRoot = if ($env:SHADOWGRAV_PROOF_DIR) { $env:SHADOWGRAV_PROOF_DIR } else { Join-Path $Root "proof/local-$Stamp" }
New-Item -ItemType Directory -Force -Path $ProofRoot | Out-Null
Set-Location $Root

@(
  "timestamp_utc=$Stamp"
  "computer=$env:COMPUTERNAME"
  "os=$([System.Runtime.InteropServices.RuntimeInformation]::OSDescription)"
  "architecture=$([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)"
  "process_architecture=$([System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture)"
  "display_session=$env:SESSIONNAME"
  "--- rustc ---"
) | Set-Content (Join-Path $ProofRoot 'machine.txt')
rustc --version --verbose 2>&1 | Tee-Object -FilePath (Join-Path $ProofRoot 'machine.txt') -Append
'--- cargo ---' | Add-Content (Join-Path $ProofRoot 'machine.txt')
cargo --version 2>&1 | Tee-Object -FilePath (Join-Path $ProofRoot 'machine.txt') -Append

cargo test --release --locked --workspace 2>&1 | Tee-Object -FilePath (Join-Path $ProofRoot 'workspace-tests.txt')
$TestCode = $LASTEXITCODE
cargo build --release --locked -p shadowgrav-ui 2>&1 | Tee-Object -FilePath (Join-Path $ProofRoot 'ui-build.txt')
$BuildCode = $LASTEXITCODE
"workspace_test_exit=$TestCode`nui_build_exit=$BuildCode" | Set-Content (Join-Path $ProofRoot 'result.txt')

if ($LaunchUI -and $TestCode -eq 0 -and $BuildCode -eq 0) {
  'Launching the native window. Close it normally after checking MACHINE_PROOF.md.' | Set-Content (Join-Path $ProofRoot 'ui-launch.txt')
  cargo run --release --locked -p shadowgrav-ui 2>&1 | Tee-Object -FilePath (Join-Path $ProofRoot 'ui-launch.txt') -Append
  $LaunchCode = $LASTEXITCODE
  "ui_launch_exit=$LaunchCode" | Add-Content (Join-Path $ProofRoot 'result.txt')
}

@"
Shadowgrav local machine proof
==============================
Evidence directory: $ProofRoot

Captured: machine.txt, workspace-tests.txt, ui-build.txt, result.txt
For native-window evidence, run with -LaunchUI on a desktop session. Verify the checklist
in MACHINE_PROOF.md and save screenshots into this directory. Compilation alone is not
an actual UI proof.

Exit codes: workspace tests=$TestCode, UI build=$BuildCode, UI launch=$LaunchCode
"@ | Set-Content (Join-Path $ProofRoot 'README.txt')

if ($TestCode -ne 0 -or $BuildCode -ne 0 -or $LaunchCode -ne 0) { exit 1 }
