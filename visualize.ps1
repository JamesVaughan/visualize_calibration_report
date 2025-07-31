# Calibration Report Visualizer - PowerShell Helper Script
# Usage: .\visualize.ps1 -InputFile "path\to\CalibrationReport.csv" -Command "summary"

param(
    [Parameter(Mandatory=$true)]
    [string]$InputFile,
    
    [Parameter(Mandatory=$true)]
    [ValidateSet("summary", "error-convergence", "value-evolution", "error-distribution")]
    [string]$Command,
    
    [string]$OutputDir = "output",
    [string]$Filter = "",
    [int]$MaxVars = 20
)

$exePath = Join-Path $PSScriptRoot "target\release\visualize_calibration_report.exe"

if (-not (Test-Path $exePath)) {
    Write-Error "Executable not found at $exePath. Please run 'cargo build --release' first."
    exit 1
}

if (-not (Test-Path $InputFile)) {
    Write-Error "Input file not found: $InputFile"
    exit 1
}

$args = @("--input", $InputFile, "--output", $OutputDir)

switch ($Command) {
    "summary" {
        $args += "summary"
    }
    "error-convergence" {
        $args += "error-convergence"
        if ($Filter) { $args += @("--filter", $Filter) }
        $args += @("--max-vars", $MaxVars)
    }
    "value-evolution" {
        $args += "value-evolution"
        if ($Filter) { $args += @("--filter", $Filter) }
        $args += @("--max-vars", $MaxVars)
    }
    "error-distribution" {
        $args += "error-distribution"
    }
}

Write-Host "Running: $exePath $($args -join ' ')"
& $exePath @args

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Command completed successfully!" -ForegroundColor Green
    if ($Command -ne "summary" -and (Test-Path $OutputDir)) {
        Write-Host "Generated files in: $OutputDir" -ForegroundColor Cyan
        Get-ChildItem $OutputDir -Filter "*.png" | ForEach-Object {
            Write-Host "  - $($_.Name)" -ForegroundColor Gray
        }
    }
} else {
    Write-Error "Command failed with exit code $LASTEXITCODE"
}
