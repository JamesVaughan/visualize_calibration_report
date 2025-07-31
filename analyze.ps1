# Comprehensive Calibration Analysis Script
# This script demonstrates how to perform a complete analysis of a calibration report

param(
    [Parameter(Mandatory=$true)]
    [string]$CalibrationFile
)

$ErrorActionPreference = "Stop"

Write-Host "🔍 Comprehensive Calibration Analysis" -ForegroundColor Cyan
Write-Host "=====================================`n" -ForegroundColor Cyan

$exe = Join-Path $PSScriptRoot "target\release\visualize_calibration_report.exe"

if (-not (Test-Path $exe)) {
    Write-Error "Please build the project first: cargo build --release"
    exit 1
}

if (-not (Test-Path $CalibrationFile)) {
    Write-Error "Calibration file not found: $CalibrationFile"
    exit 1
}

Write-Host "📊 Step 1: Generating Summary Statistics" -ForegroundColor Yellow
& $exe --input $CalibrationFile summary
Write-Host ""

Write-Host "📈 Step 2: Overall Error Distribution" -ForegroundColor Yellow
& $exe --input $CalibrationFile error-distribution
Write-Host "✓ Saved: output\error_distribution.png`n" -ForegroundColor Green

Write-Host "🚗 Step 3: Auto Ownership & Vehicle Analysis" -ForegroundColor Yellow
& $exe --input $CalibrationFile error-convergence --filter "AutoOwnership,Auto-" --max-vars 10
Write-Host "✓ Saved: output\error_convergence.png`n" -ForegroundColor Green

& $exe --input $CalibrationFile value-evolution --filter "AutoOwnership,Auto-" --max-vars 10
Write-Host "✓ Saved: output\value_evolution.png`n" -ForegroundColor Green

Write-Host "🚌 Step 4: Transit Mode Analysis (WAT/DAT/PAT)" -ForegroundColor Yellow
& $exe --input $CalibrationFile --output output\transit error-convergence --filter "WAT,DAT,PAT" --max-vars 15
Write-Host "✓ Saved: output\transit\error_convergence.png`n" -ForegroundColor Green

& $exe --input $CalibrationFile --output output\transit value-evolution --filter "WAT,DAT,PAT" --max-vars 15
Write-Host "✓ Saved: output\transit\value_evolution.png`n" -ForegroundColor Green

Write-Host "🏙️ Step 5: Regional Analysis (Montreal, Laval, etc.)" -ForegroundColor Yellow
& $exe --input $CalibrationFile --output output\regional error-convergence --filter "Montreal,Laval,South,North" --max-vars 12
Write-Host "✓ Saved: output\regional\error_convergence.png`n" -ForegroundColor Green

Write-Host "👥 Step 6: Employment Category Analysis" -ForegroundColor Yellow
& $exe --input $CalibrationFile --output output\employment error-convergence --filter "Professional,General,Sales,Manufacturing,Students" --max-vars 12
Write-Host "✓ Saved: output\employment\error_convergence.png`n" -ForegroundColor Green

Write-Host "📋 Analysis Complete!" -ForegroundColor Green
Write-Host "=================" -ForegroundColor Green
Write-Host "Generated files:" -ForegroundColor White
Get-ChildItem "output" -Recurse -Filter "*.png" | ForEach-Object {
    Write-Host "  📄 $($_.FullName.Substring((Get-Location).Path.Length + 1))" -ForegroundColor Gray
}

Write-Host "`n💡 Tips for Interpretation:" -ForegroundColor Cyan
Write-Host "- Error convergence plots should trend downward for successful calibration" -ForegroundColor White
Write-Host "- Value evolution plots show how parameters are being adjusted" -ForegroundColor White
Write-Host "- Large final errors may indicate calibration issues or conflicting constraints" -ForegroundColor White
Write-Host "- Review the summary for overall calibration performance" -ForegroundColor White
