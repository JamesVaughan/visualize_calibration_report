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

struct CalibrationApp {
    records: Vec<CalibrationRecord>,
    error_columns: Vec<String>,
    value_columns: Vec<String>,
    variable_names: Vec<String>, // Base variable names without Error:/Value: prefix
    
    // UI State
    file_path: String,
    file_loaded: bool,
    loading_error: Option<String>,
    
    // Plot selection - now per variable with option for error/value/both
    selected_vars: Vec<bool>,
    show_error_for_var: Vec<bool>,
    show_value_for_var: Vec<bool>,
    filter_text: String,
    
    // Statistics
    show_summary: bool,
    final_errors: Vec<(String, f64)>,
    total_error_trend: Vec<(f64, f64)>,
    
    // UI filters
    max_vars_to_show: usize,
    show_only_filtered: bool,
}

impl Default for CalibrationApp {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            error_columns: Vec::new(),
            value_columns: Vec::new(),
            variable_names: Vec::new(),
            file_path: String::new(),
            file_loaded: false,
            loading_error: None,
            selected_vars: Vec::new(),
            show_error_for_var: Vec::new(),
            show_value_for_var: Vec::new(),
            filter_text: String::new(),
            show_summary: false,
            final_errors: Vec::new(),
            total_error_trend: Vec::new(),
            max_vars_to_show: 50,
            show_only_filtered: false,
        }
    }
}

impl CalibrationApp {
    fn load_file(&mut self, path: String) -> Result<()> {
        println!("Starting to load file: {}", path);
        
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {}", path))?;
        
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
                println!("Loaded {} records...", record_count);
            }
        }
        
        println!("Finished loading {} records", record_count);
        
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
        
        // Calculate summary statistics
        let final_errors = self.calculate_final_errors(&records, &error_columns);
        let total_error_trend = self.calculate_total_error_trend(&records, &error_columns);
        
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
        let show_error_for_var = vec![true; variable_names.len()];
        let show_value_for_var = vec![false; variable_names.len()];
        
        // Update state
        self.records = records;
        self.error_columns = error_columns;
        self.value_columns = value_columns;
        self.variable_names = variable_names;
        self.selected_vars = selected_vars;
        self.show_error_for_var = show_error_for_var;
        self.show_value_for_var = show_value_for_var;
        self.final_errors = final_errors;
        self.total_error_trend = total_error_trend;
        self.file_loaded = true;
        self.loading_error = None;
        
        Ok(())
    }
    
    fn calculate_final_errors(&self, records: &[CalibrationRecord], error_columns: &[String]) -> Vec<(String, f64)> {
        if let Some(last_record) = records.last() {
            let mut final_errors: Vec<(String, f64)> = error_columns
                .iter()
                .filter_map(|col| {
                    last_record.data.get(col).map(|&val| (col.clone(), val.abs()))
                })
                .collect();
            
            final_errors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            final_errors
        } else {
            Vec::new()
        }
    }
    
    fn calculate_total_error_trend(&self, records: &[CalibrationRecord], error_columns: &[String]) -> Vec<(f64, f64)> {
        records
            .iter()
            .map(|record| {
                let total_error: f64 = error_columns
                    .iter()
                    .filter_map(|col| record.data.get(col).map(|&val| val.abs()))
                    .sum();
                (record.iteration as f64, total_error)
            })
            .collect()
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
            .take(self.max_vars_to_show)
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
                
                if !self.file_path.is_empty() {
                    if ui.button("🔄 Reload").clicked() {
                        if let Err(e) = self.load_file(self.file_path.clone()) {
                            self.loading_error = Some(e.to_string());
                            self.file_loaded = false;
                        }
                    }
                }
            });
            
            if let Some(error) = &self.loading_error {
                ui.colored_label(Color32::RED, format!("❌ Error: {}", error));
            }
            
            if !self.file_loaded {
                ui.colored_label(Color32::GRAY, "Load a calibration CSV file to begin analysis");
                return;
            }
            
            ui.separator();
            
            // Summary section
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_summary, "📈 Show Summary");
                if self.show_summary {
                    ui.label(format!("📊 {} iterations, {} error variables, {} value variables", 
                        self.records.len(), self.error_columns.len(), self.value_columns.len()));
                }
            });
            
            if self.show_summary {
                ui.group(|ui| {
                    ui.label(RichText::new("Top 10 Variables by Final Error:").strong());
                    for (i, (var, error)) in self.final_errors.iter().take(10).enumerate() {
                        let clean_name = var.strip_prefix("Error:").unwrap_or(var);
                        ui.label(format!("  {}. {} = {:.6}", i + 1, clean_name, error));
                    }
                    
                    if let (Some(first), Some(last)) = (self.total_error_trend.first(), self.total_error_trend.last()) {
                        let improvement = ((first.1 - last.1) / first.1) * 100.0;
                        ui.separator();
                        ui.label(format!("📉 Total Error: {:.6} → {:.6} ({:.2}% change)", 
                            first.1, last.1, improvement));
                    }
                });
            }
            
            ui.separator();
            
            // Plot type selection - removed since we now show both per variable
            
            // Filter controls
            ui.horizontal(|ui| {
                ui.label("🔍 Filter:");
                ui.text_edit_singleline(&mut self.filter_text);
                ui.label("Max vars:");
                ui.add(egui::DragValue::new(&mut self.max_vars_to_show).range(1..=100));
                ui.checkbox(&mut self.show_only_filtered, "Show only filtered");
            });
            
            ui.separator();
            
            // Variable selection and plotting
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_variables_section(ui);
                
                // Total error trend (always show if data is loaded)
                if !self.total_error_trend.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("📊 Total Absolute Error Over Time").heading());
                    
                    let points: PlotPoints = self.total_error_trend.iter().map(|(x, y)| [*x, *y]).collect();
                    let line = Line::new(points).color(Color32::RED).width(2.0);
                    
                    Plot::new("total_error_plot")
                        .view_aspect(2.0)
                        .height(200.0)
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                }
            });
        });
    }
}

impl CalibrationApp {
    fn show_variables_section(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("� Variables").heading());
        
        let filtered_vars = self.filter_columns(&self.variable_names);
        
        if filtered_vars.is_empty() {
            ui.label("No variables match the current filter");
            return;
        }
        
        // Variable selection and options
        let mut temp_selections = Vec::new();
        let mut temp_error_options = Vec::new();
        let mut temp_value_options = Vec::new();
        
        for var_name in filtered_vars.iter() {
            if let Some(var_index) = self.variable_names.iter().position(|x| x == var_name) {
                if var_index >= self.selected_vars.len() {
                    continue;
                }
                
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Main checkbox to select the variable
                        let mut selected = self.selected_vars[var_index];
                        if ui.checkbox(&mut selected, format!("📈 {}", var_name)).changed() {
                            temp_selections.push((var_index, selected));
                        }
                        
                        if selected {
                            ui.separator();
                            
                            // Error checkbox (only if error column exists)
                            if self.has_error_column(var_name) {
                                let mut show_error = self.show_error_for_var[var_index];
                                if ui.checkbox(&mut show_error, "🔴 Error").changed() {
                                    temp_error_options.push((var_index, show_error));
                                }
                            }
                            
                            // Value checkbox (only if value column exists)
                            if self.has_value_column(var_name) {
                                let mut show_value = self.show_value_for_var[var_index];
                                if ui.checkbox(&mut show_value, "🔵 Value").changed() {
                                    temp_value_options.push((var_index, show_value));
                                }
                            }
                        }
                    });
                });
            }
        }
        
        // Apply all changes
        for (var_index, selected) in temp_selections {
            self.selected_vars[var_index] = selected;
        }
        
        for (var_index, show_error) in temp_error_options {
            self.show_error_for_var[var_index] = show_error;
        }
        
        for (var_index, show_value) in temp_value_options {
            self.show_value_for_var[var_index] = show_value;
        }
        
        // Plot selected variables
        let selected_variables: Vec<(usize, &String)> = self.variable_names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < self.selected_vars.len() && self.selected_vars[*i])
            .collect();
        
        if !selected_variables.is_empty() {
            ui.separator();
            ui.label(RichText::new("📈 Selected Variables Plot").heading());
            
            let colors = [
                Color32::RED, Color32::BLUE, Color32::GREEN, Color32::from_rgb(255, 165, 0),
                Color32::from_rgb(128, 0, 128), Color32::from_rgb(165, 42, 42),
                Color32::YELLOW, Color32::from_rgb(255, 192, 203), Color32::DARK_GRAY, Color32::BROWN,
            ];
            
            Plot::new("variables_plot")
                .view_aspect(2.0)
                .height(400.0)
                .legend(egui_plot::Legend::default())
                .show(ui, |plot_ui| {
                    let mut plot_idx = 0;
                    
                    for (var_index, var_name) in &selected_variables {
                        // Plot error if selected and available
                        if *var_index < self.show_error_for_var.len() && 
                           self.show_error_for_var[*var_index] && 
                           self.has_error_column(var_name) {
                            
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
                                    .name(format!("{} (Error)", var_name));
                                
                                plot_ui.line(line);
                                plot_idx += 1;
                            }
                        }
                        
                        // Plot value if selected and available
                        if *var_index < self.show_value_for_var.len() && 
                           self.show_value_for_var[*var_index] && 
                           self.has_value_column(var_name) {
                            
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
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .name(format!("{} (Value)", var_name));
                                
                                plot_ui.line(line);
                                plot_idx += 1;
                            }
                        }
                    }
                });
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 800.0])
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
