# Calibration Report Visualizer

A professional interactive GUI tool for visualizing calibration reports from the XTMF Automated Calibration Framework. This application provides comprehensive analysis and visualization capabilities for calibration convergence data.

## Features

### 📊 Interactive GUI
- **Modern Interface**: Built with egui/eframe for a responsive, native-feeling GUI
- **Dynamic Layout**: Automatically adjusts column layout based on window size
- **Theme Support**: Automatically detects and respects system dark/light mode preferences
- **File Management**: Browse and load CSV files with native file dialogs

### 📈 Advanced Visualization
- **Side-by-Side Plotting**: Separate error and value plots for clear comparison
- **Color-Coded Variables**: Each selected variable gets a unique color across UI and plots
- **Interactive Plots**: Zoom, pan, and explore data with full interactivity
- **Professional Legends**: Positioned legends with background styling
- **Axis Labels**: Clear iteration and value/error axis labeling

### 🎛️ Variable Management
- **Smart Filtering**: Filter variables by name with comma-separated search terms
- **Multi-Column Selection**: Dynamic checkbox layout optimized for screen width
- **Visual Color Mapping**: Checkbox backgrounds match graph line colors for selected variables
- **Bulk Operations**: Select all filtered variables or unselect all with one click
- **Real-time Updates**: Plot view resets automatically when selection changes

### 💾 Export Capabilities
- **CSV Export**: Right-click context menus to save plot data as CSV files
- **High-Resolution Images**: Export plots as PNG images with 1600x1200 resolution
- **Viewport-Aware Export**: Exported images respect current zoom/pan settings
- **Theme-Consistent Export**: Exported images match current UI theme (dark/light)
- **Professional Output**: Publication-quality images with proper legends and styling

### 🔧 Technical Features
- **Large File Support**: Efficient handling of large calibration datasets
- **Progress Feedback**: Loading progress indication for large files
- **Error Handling**: Comprehensive error reporting and user feedback
- **Memory Efficient**: Optimized data structures for performance
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Installation

### Requirements
- Rust 1.70+ with Cargo
- Windows, macOS, or Linux

### Building from Source
```bash
# Clone the repository
git clone <repository-url>
cd visualize_calibration_report

# Build in release mode for optimal performance
cargo build --release

# Run the application
cargo run --release
```

### Running
```bash
# From the project directory
cargo run --release

# Or run the built executable directly
./target/release/visualize_calibration_report
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
