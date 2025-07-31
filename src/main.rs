use anyhow::{Context, Result};
use csv::ReaderBuilder;
use egui::{Color32, RichText, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;

#[derive(Debug, Deserialize)]
struct CalibrationRecord {
    #[serde(rename = "Iteration")]
    iteration: u32,
    #[serde(flatten)]
    data: HashMap<String, f64>,
}

#[derive(Default)]
struct CalibrationApp {
    records: Vec<CalibrationRecord>,
    error_columns: Vec<String>,
    value_columns: Vec<String>,
    variable_names: Vec<String>, // Base variable names without Error:/Value: prefix
    
    // UI State
    file_path: String,
    file_loaded: bool,
    loading_error: Option<String>,
    
    // Plot selection - simplified to just variable selection
    selected_vars: Vec<bool>,
    prev_selected_vars: Vec<bool>, // Track previous selection to detect changes
    filter_text: String,
}


impl CalibrationApp {
    fn load_file(&mut self, path: String) -> Result<()> {
        println!("Starting to load file: {path}");
        
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {path}"))?;
        
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);
        
        let mut records: Vec<CalibrationRecord> = Vec::new();
        let mut record_count = 0;
        
        for result in rdr.deserialize() {
            let record: CalibrationRecord = result
                .with_context(|| format!("Failed to parse CSV record at line {}", record_count + 2))?;
            records.push(record);
            record_count += 1;
            
            // Add progress feedback for large files
            if record_count % 100 == 0 {
                println!("Loaded {record_count} records...");
            }
        }
        
        println!("Finished loading {record_count} records");
        
        if records.is_empty() {
            return Err(anyhow::anyhow!("No records found in file"));
        }
        
        // Extract column names
        let error_columns: Vec<String> = records[0]
            .data
            .keys()
            .filter(|k| k.starts_with("Error:"))
            .cloned()
            .collect();
        
        let value_columns: Vec<String> = records[0]
            .data
            .keys()
            .filter(|k| k.starts_with("Value:"))
            .cloned()
            .collect();
        
        // Create unified variable names (base names without Error:/Value: prefix)
        let mut variable_names = std::collections::HashSet::new();
        
        for col in &error_columns {
            if let Some(base_name) = col.strip_prefix("Error:") {
                variable_names.insert(base_name.trim().to_string());
            }
        }
        
        for col in &value_columns {
            if let Some(base_name) = col.strip_prefix("Value:") {
                variable_names.insert(base_name.trim().to_string());
            }
        }
        
        let mut variable_names: Vec<String> = variable_names.into_iter().collect();
        variable_names.sort();
        
        // Initialize selection vectors
        let selected_vars = vec![false; variable_names.len()];
        let prev_selected_vars = vec![false; variable_names.len()];
        
        // Update state
        self.records = records;
        self.error_columns = error_columns;
        self.value_columns = value_columns;
        self.variable_names = variable_names;
        self.selected_vars = selected_vars;
        self.prev_selected_vars = prev_selected_vars;
        self.file_loaded = true;
        self.loading_error = None;
        
        Ok(())
    }
    
    fn filter_columns(&self, columns: &[String]) -> Vec<String> {
        let filtered: Vec<String> = columns
            .iter()
            .filter(|col| {
                if self.filter_text.is_empty() {
                    true
                } else {
                    let filter_terms: Vec<&str> = self.filter_text.split(',').map(|s| s.trim()).collect();
                    filter_terms.iter().any(|term| col.to_lowercase().contains(&term.to_lowercase()))
                }
            })
            .cloned()
            .collect();
        
        filtered
    }
    
    fn has_error_column(&self, var_name: &str) -> bool {
        self.error_columns.iter().any(|col| {
            col.strip_prefix("Error:").map(|s| s.trim()) == Some(var_name)
        })
    }
    
    fn has_value_column(&self, var_name: &str) -> bool {
        self.value_columns.iter().any(|col| {
            col.strip_prefix("Value:").map(|s| s.trim()) == Some(var_name)
        })
    }
    
    fn get_error_column_name(&self, var_name: &str) -> Option<String> {
        self.error_columns.iter().find(|col| {
            col.strip_prefix("Error:").map(|s| s.trim()) == Some(var_name)
        }).cloned()
    }
    
    fn get_value_column_name(&self, var_name: &str) -> Option<String> {
        self.value_columns.iter().find(|col| {
            col.strip_prefix("Value:").map(|s| s.trim()) == Some(var_name)
        }).cloned()
    }
}

impl eframe::App for CalibrationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📊 Calibration Report Visualizer");
            ui.separator();
            
            // File loading section
            ui.horizontal(|ui| {
                ui.label("CSV File:");
                ui.text_edit_singleline(&mut self.file_path);
                if ui.button("📁 Browse & Load File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV Files", &["csv"])
                        .add_filter("All Files", &["*"])
                        .set_title("Select Calibration CSV File")
                        .pick_file()
                    {
                        self.file_path = path.display().to_string();
                        if let Err(e) = self.load_file(self.file_path.clone()) {
                            self.loading_error = Some(e.to_string());
                            self.file_loaded = false;
                        }
                    }
                }
                
                if !self.file_path.is_empty() && ui.button("🔄 Reload").clicked() {
                    if let Err(e) = self.load_file(self.file_path.clone()) {
                        self.loading_error = Some(e.to_string());
                        self.file_loaded = false;
                    }
                }
            });
            
            if let Some(error) = &self.loading_error {
                ui.colored_label(Color32::RED, format!("❌ Error: {error}"));
            }
            
            if !self.file_loaded {
                ui.colored_label(Color32::GRAY, "Load a calibration CSV file to begin analysis");
                return;
            }
            
            ui.separator();
            
            // Filter controls
            ui.horizontal(|ui| {
                ui.label("🔍 Filter:");
                ui.text_edit_singleline(&mut self.filter_text);
            });
            
            ui.separator();
            
            // Variable selection and plotting
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_variables_section(ui);
            });
        });
    }
}

impl CalibrationApp {
    fn show_variables_section(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Variables").heading());
        
        let filtered_vars = self.filter_columns(&self.variable_names);
        
        if filtered_vars.is_empty() {
            ui.label("No variables match the current filter");
            return;
        }
        
        // Calculate pagination
        let total_vars = filtered_vars.len();
        
        // Show variable count
        ui.horizontal(|ui| {
            ui.label(format!("� Showing {total_vars} variables"));
        });
        
        ui.separator();
        // Variable selection and options in scrollable area
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                // Calculate optimal number of columns based on available width
                // Estimate column width: checkbox + text + padding (~200px per column)
                let available_width = ui.available_width();
                let estimated_column_width = 200.0;
                let columns_count = ((available_width / estimated_column_width) as usize).clamp(1, 6); // Between 1-6 columns
                let vars_per_column = filtered_vars.len().div_ceil(columns_count);
                
                ui.horizontal_top(|ui| {
                    for col_idx in 0..columns_count {
                        let start = col_idx * vars_per_column;
                        let end = ((col_idx + 1) * vars_per_column).min(filtered_vars.len());
                        
                        if start >= filtered_vars.len() {
                            break;
                        }                        
                        ui.vertical(|ui| {
                            for var_name in &filtered_vars[start..end] {
                                if let Some(var_index) = self.variable_names.iter().position(|x| x == var_name) {
                                    if var_index >= self.selected_vars.len() {
                                        continue;
                                    }
                                    
                                    ui.group(|ui| {
                                        ui.vertical(|ui| {
                                            // Main checkbox to select the variable
                                            let mut selected = self.selected_vars[var_index];
                                            if ui.checkbox(&mut selected, format!("📈 {var_name}")).changed() {
                                                self.selected_vars[var_index] = selected;
                                            }
                                        });
                                    });
                                    
                                    ui.add_space(2.0); // Small spacing between variables
                                }
                            }
                        });
                        
                        // Add column separator
                        if col_idx < columns_count - 1 && end < filtered_vars.len() {
                            ui.separator();
                        }
                    }
                });
            });
        
        ui.separator();
        
        // Show selection summary
        let selected_count = self.selected_vars.iter().filter(|&&x| x).count();
        ui.horizontal(|ui| {
            ui.label(format!("📊 Selected: {selected_count} variables"));
            
            if ui.button("✅ Select All Filtered").clicked() {
                for var_name in &filtered_vars {
                    if let Some(var_index) = self.variable_names.iter().position(|x| x == var_name) {
                        if var_index < self.selected_vars.len() {
                            self.selected_vars[var_index] = true;
                        }
                    }
                }
            }
            
            if ui.button("❌ Unselect All").clicked() {
                for selection in &mut self.selected_vars {
                    *selection = false;
                }
            }
        });
        
        // Plot selected variables
        let selected_variables: Vec<(usize, &String)> = self.variable_names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < self.selected_vars.len() && self.selected_vars[*i])
            .collect();
        
        // Check if selection has changed to reset view
        let selection_changed = self.selected_vars != self.prev_selected_vars;
        if selection_changed {
            self.prev_selected_vars = self.selected_vars.clone();
        }
        
        if !selected_variables.is_empty() {
            ui.separator();
            ui.label(RichText::new("📈 Selected Variables Plots").heading());
            ui.separator();
            
            let colors = [
                Color32::RED, Color32::BLUE, Color32::GREEN, Color32::from_rgb(255, 165, 0),
                Color32::from_rgb(128, 0, 128), Color32::from_rgb(165, 42, 42),
                Color32::YELLOW, Color32::from_rgb(255, 192, 203), Color32::DARK_GRAY, Color32::BROWN,
            ];
            
            // Check if we have any error or value data to show
            let has_error_data = selected_variables.iter().any(|(_, var_name)| {
                self.has_error_column(var_name)
            });
            
            let has_value_data = selected_variables.iter().any(|(_, var_name)| {
                self.has_value_column(var_name)
            });
            
            // Show plots side by side
            ui.horizontal(|ui| {
                let total_width = ui.available_width();
                let plot_width = (total_width - 40.0) * 0.5;
                ui.add_space(5.0); // Extra spacing between plots
                // Error plot (left side)
                if has_error_data {
                    ui.vertical(|ui| {
                        ui.add_space(5.0); // Increased top padding
                        ui.label(RichText::new("🔴 Error Convergence").strong());
                        ui.add_space(2.0); // Increased spacing after label
                        
                        let mut error_plot = Plot::new("error_plot")
                            .view_aspect(2.0) // Increased aspect ratio for more horizontal space
                            .height(380.0) // Increased height
                            .width(plot_width) // Reduced width to add margins
                            .legend(egui_plot::Legend::default())
                            .x_axis_label("Iteration")
                            .y_axis_label("Absolute Error");
                        
                        // Reset view if selection changed
                        if selection_changed {
                            error_plot = error_plot.auto_bounds([true, true].into()).reset();
                        }
                        
                        error_plot.show(ui, |plot_ui| {
                                let mut plot_idx = 0;
                                
                                for (_, var_name) in &selected_variables {
                                    if self.has_error_column(var_name) {
                                        if let Some(error_col) = self.get_error_column_name(var_name) {
                                            let points: PlotPoints = self.records
                                                .iter()
                                                .filter_map(|r| {
                                                    r.data.get(&error_col).map(|&val| [r.iteration as f64, val.abs()])
                                                })
                                                .collect();
                                            
                                            let line = Line::new(points)
                                                .color(colors[plot_idx % colors.len()])
                                                .width(2.0)
                                                .name(var_name);
                                            
                                            plot_ui.line(line);
                                            plot_idx += 1;
                                        }
                                    }
                                }
                            });
                    });
                }
                
                // Add spacing between plots
                if has_error_data && has_value_data {
                    ui.add_space(2.0); // Extra spacing between plots
                    ui.separator();
                    ui.add_space(2.0); // Extra spacing between plots
                }
                
                // Value plot (right side)
                if has_value_data {
                    ui.vertical(|ui| {
                        ui.add_space(5.0); // Increased top padding
                        ui.label(RichText::new("🔵 Value Evolution").strong());
                        ui.add_space(2.0); // Increased spacing after label
                        
                        let mut value_plot = Plot::new("value_plot")
                            .view_aspect(2.0) // Increased aspect ratio for more horizontal space
                            .height(380.0) // Increased height
                            .width(plot_width) // Reduced width to add margins
                            .legend(egui_plot::Legend::default())
                            .x_axis_label("Iteration")
                            .y_axis_label("Value");
                        
                        // Reset view if selection changed
                        if selection_changed {
                            value_plot = value_plot.auto_bounds([true, true].into()).reset();
                        }
                        
                        value_plot.show(ui, |plot_ui| {
                                let mut plot_idx = 0;
                                
                                for (_, var_name) in &selected_variables {
                                    if self.has_value_column(var_name) {
                                        if let Some(value_col) = self.get_value_column_name(var_name) {
                                            let points: PlotPoints = self.records
                                                .iter()
                                                .filter_map(|r| {
                                                    r.data.get(&value_col).map(|&val| [r.iteration as f64, val])
                                                })
                                                .collect();
                                            
                                            let line = Line::new(points)
                                                .color(colors[plot_idx % colors.len()])
                                                .width(2.0)
                                                .name(var_name);
                                            
                                            plot_ui.line(line);
                                            plot_idx += 1;
                                        }
                                    }
                                }
                            });
                    });
                    ui.add_space(100.0); // Extra spacing between plots
                }
            });
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_title("Calibration Report Visualizer"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Calibration Report Visualizer",
        options,
        Box::new(|_cc| {
            // Create app with no file operations at startup
            let app = CalibrationApp::default();
            Ok(Box::new(app))
        }),
    )
}
