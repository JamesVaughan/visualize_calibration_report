@echo off
REM Calibration Report Visualizer - Batch Helper Script
REM Usage: visualize.bat "path\to\CalibrationReport.csv" summary
REM Usage: visualize.bat "path\to\CalibrationReport.csv" error-convergence [filter] [max_vars]

if "%~1"=="" (
    echo Usage: %0 ^<input_file^> ^<command^> [filter] [max_vars]
    echo Commands: summary, error-convergence, value-evolution, error-distribution
    echo Example: %0 "Z:\CalibrationReport.csv" summary
    echo Example: %0 "Z:\CalibrationReport.csv" error-convergence "AutoOwnership,DAT" 15
    exit /b 1
)

set INPUT_FILE=%~1
set COMMAND=%~2

REM For summary and error-distribution, no additional args needed
if "%COMMAND%"=="summary" goto :run_command
if "%COMMAND%"=="error-distribution" goto :run_command

REM For error-convergence and value-evolution, parse filter and max_vars
set FILTER=%~3
set MAX_VARS=%~4

REM If third argument is a number, treat it as max_vars and no filter
echo %FILTER% | findstr /r "^[0-9][0-9]*$" >nul
if %ERRORLEVEL% equ 0 (
    set MAX_VARS=%FILTER%
    set FILTER=
)

if "%MAX_VARS%"=="" set MAX_VARS=20

:run_command
set EXE_PATH=%~dp0target\release\visualize_calibration_report.exe

if not exist "%EXE_PATH%" (
    echo Error: Executable not found at %EXE_PATH%
    echo Please run 'cargo build --release' first.
    exit /b 1
)

if not exist "%INPUT_FILE%" (
    echo Error: Input file not found: %INPUT_FILE%
    exit /b 1
)

echo Running calibration report visualizer...

if "%COMMAND%"=="summary" (
    "%EXE_PATH%" --input "%INPUT_FILE%" summary
) else if "%COMMAND%"=="error-convergence" (
    if "%FILTER%"=="" (
        "%EXE_PATH%" --input "%INPUT_FILE%" error-convergence --max-vars %MAX_VARS%
    ) else (
        "%EXE_PATH%" --input "%INPUT_FILE%" error-convergence --filter "%FILTER%" --max-vars %MAX_VARS%
    )
) else if "%COMMAND%"=="value-evolution" (
    if "%FILTER%"=="" (
        "%EXE_PATH%" --input "%INPUT_FILE%" value-evolution --max-vars %MAX_VARS%
    ) else (
        "%EXE_PATH%" --input "%INPUT_FILE%" value-evolution --filter "%FILTER%" --max-vars %MAX_VARS%
    )
) else if "%COMMAND%"=="error-distribution" (
    "%EXE_PATH%" --input "%INPUT_FILE%" error-distribution
) else (
    echo Error: Unknown command '%COMMAND%'
    echo Valid commands: summary, error-convergence, value-evolution, error-distribution
    exit /b 1
)

if %ERRORLEVEL% equ 0 (
    echo.
    echo ✓ Command completed successfully!
    if exist "output" (
        echo Generated files in output directory:
        dir /b output\*.png 2>nul
    )
) else (
    echo.
    echo ✗ Command failed with error code %ERRORLEVEL%
)
