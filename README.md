# Calibration Report Visualizer

A Rust tool for visualizing calibration reports from the Tasha2Engine. This tool can process CSV files containing calibration data with iterations, error values, and parameter values.

## Features

- **Error Convergence Analysis**: Plot how error values change over iterations
- **Parameter Value Evolution**: Visualize how parameter values evolve during calibration
- **Summary Statistics**: Generate comprehensive summaries of the calibration process
- **Error Distribution**: Plot total absolute error distribution over iterations

## Installation

```bash
cargo build --release
```

## Usage

### Basic Commands

```bash
# Generate a summary of the calibration process
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv summary

# Plot error convergence for all variables (limited to top 20)
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv error-convergence

# Plot parameter value evolution
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv value-evolution

# Plot total error distribution over iterations
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv error-distribution
```

### Advanced Usage

```bash
# Filter specific variables and limit the number plotted
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv error-convergence --filter "AutoOwnership,DAT,WAT" --max-vars 15

# Specify custom output directory
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv --output my_plots summary

# Plot specific parameter evolution
./target/release/visualize_calibration_report --input path/to/CalibrationReport.csv value-evolution --filter "Montreal,Laval" --max-vars 8
```

## CSV File Format

The tool expects a CSV file with:
- An "Iteration" column containing iteration numbers
- Columns starting with "Error:" followed by variable names (e.g., "Error:AutoOwnership-1")
- Columns starting with "Value:" followed by variable names (e.g., "Value:AutoOwnership-1")

Example:
```csv
Iteration,Error:AutoOwnership-1,Error:AutoOwnership-2,Value:AutoOwnership-1,Value:AutoOwnership-2
0,0.1,-0.05,1.2,0.8
1,0.08,-0.03,1.15,0.82
...
```

## Output

The tool generates PNG images in the specified output directory (default: `output/`):
- `error_convergence.png`: Line plot showing how errors change over iterations
- `value_evolution.png`: Line plot showing how parameter values evolve
- `error_distribution.png`: Line plot of total absolute error over iterations

## Options

- `--input` (`-i`): Path to the calibration report CSV file (required)
- `--output` (`-o`): Output directory for plots (default: "output")
- `--filter` (`-f`): Comma-separated list of terms to filter variables
- `--max-vars` (`-m`): Maximum number of variables to plot (default: 20)

## Examples

### Analyzing Auto Ownership Variables
```bash
./target/release/visualize_calibration_report -i calibration.csv error-convergence -f "AutoOwnership" -m 5
```

### Quick Overview
```bash
./target/release/visualize_calibration_report -i calibration.csv summary
./target/release/visualize_calibration_report -i calibration.csv error-distribution
```

### Detailed Analysis of Transit Variables
```bash
./target/release/visualize_calibration_report -i calibration.csv error-convergence -f "WAT,DAT,PAT" -m 15
./target/release/visualize_calibration_report -i calibration.csv value-evolution -f "WAT,DAT,PAT" -m 15
```

## Interpreting Results

### Summary Output
- **Total iterations**: Number of calibration iterations processed
- **Number of error variables**: Count of variables being calibrated
- **Top variables by final error**: Variables with highest absolute error at the end
- **Total absolute error**: Sum of all absolute errors at start vs. end
- **Improvement percentage**: Positive values indicate error reduction

### Error Convergence Plots
- X-axis: Iteration number
- Y-axis: Absolute error value
- Each line represents a different variable
- Ideally, lines should trend downward (converging to zero)

### Value Evolution Plots
- X-axis: Iteration number  
- Y-axis: Parameter value
- Shows how the calibration algorithm adjusts parameter values
- Should stabilize as calibration converges

### Error Distribution Plots
- X-axis: Iteration number
- Y-axis: Total absolute error (sum of all errors)
- Shows overall convergence progress
- Should trend downward for successful calibration
