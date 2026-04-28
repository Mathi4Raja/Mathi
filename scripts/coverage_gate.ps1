param(
	[double]$MinimumCoverage = 94.0,
	[switch]$NoClean
)

$ErrorActionPreference = 'Stop'

function Require-Command {
	param([string]$Name, [string]$InstallHint)
	if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
		Write-Error "Missing required command: $Name. $InstallHint"
	}
}

Require-Command cargo "Install Rust toolchain from https://rustup.rs"

if (-not (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
	Write-Host "Installing cargo-llvm-cov..."
	cargo install cargo-llvm-cov --locked
}

if (-not $NoClean) {
	Write-Host "Cleaning previous llvm-cov artifacts..."
	& cargo llvm-cov clean --workspace
	if ($LASTEXITCODE -ne 0) {
		Write-Error "cargo llvm-cov clean failed"
	}
}

$coverageArgs = @("llvm-cov", "--workspace", "--all-targets", "--summary-only")

Write-Host "Running coverage collection..."
$output = & cargo @coverageArgs
if ($LASTEXITCODE -ne 0) {
	Write-Error "cargo llvm-cov failed"
}

$match = [regex]::Match($output, "TOTAL\s+\d+\s+\d+\s+\d+\.\d+%")
if (-not $match.Success) {
	Write-Error "Could not find TOTAL coverage in output.`n$output"
}

$percentMatch = [regex]::Match($match.Value, "(\d+\.\d+)%")
if (-not $percentMatch.Success) {
	Write-Error "Could not parse coverage percentage from: $($match.Value)"
}

$coverage = [double]$percentMatch.Groups[1].Value
Write-Host ("Coverage total: {0:N2}%" -f $coverage)

if ($coverage -lt $MinimumCoverage) {
	Write-Error ("Coverage gate failed: {0:N2}% < {1:N2}%" -f $coverage, $MinimumCoverage)
}

Write-Host ("Coverage gate passed: {0:N2}% >= {1:N2}%" -f $coverage, $MinimumCoverage)
