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
    
    // UI State
    file_path: String,
    file_loaded: bool,
    loading_error: Option<String>,
    
    // Plot selection
    selected_error_vars: Vec<bool>,
    selected_value_vars: Vec<bool>,
    filter_text: String,
    show_errors: bool,
    show_values: bool,
    
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
            file_path: String::new(),
            file_loaded: false,
            loading_error: None,
            selected_error_vars: Vec::new(),
            selected_value_vars: Vec::new(),
            filter_text: String::new(),
            show_errors: true,
            show_values: false,
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
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {}", path))?;
        
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);
        
        let mut records: Vec<CalibrationRecord> = Vec::new();
        for result in rdr.deserialize() {
            let record: CalibrationRecord = result
                .with_context(|| "Failed to parse CSV record")?;
            records.push(record);
        }
        
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
        
        // Initialize selection vectors
        let selected_error_vars = vec![false; error_columns.len()];
        let selected_value_vars = vec![false; value_columns.len()];
        
        // Update state
        self.records = records;
        self.error_columns = error_columns;
        self.value_columns = value_columns;
        self.selected_error_vars = selected_error_vars;
        self.selected_value_vars = selected_value_vars;
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
                if ui.button("📁 Load File").clicked() {
                    if let Err(e) = self.load_file(self.file_path.clone()) {
                        self.loading_error = Some(e.to_string());
                        self.file_loaded = false;
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
            
            // Plot type selection
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_errors, "🔴 Error Convergence");
                ui.checkbox(&mut self.show_values, "🔵 Value Evolution");
            });
            
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
                if self.show_errors {
                    self.show_error_section(ui);
                }
                
                if self.show_values {
                    self.show_value_section(ui);
                }
                
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
    fn show_error_section(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("🔴 Error Variables").heading());
        
        let filtered_columns = self.filter_columns(&self.error_columns);
        
        if filtered_columns.is_empty() {
            ui.label("No error variables match the current filter");
            return;
        }
        
        // Variable selection checkboxes
        let mut temp_selections = Vec::new();
        
        ui.horizontal_wrapped(|ui| {
            for col in filtered_columns.iter() {
                let clean_name = col.strip_prefix("Error:").unwrap_or(col);
                let mut selected = false;
                if let Some(original_index) = self.error_columns.iter().position(|x| x == col) {
                    if original_index < self.selected_error_vars.len() {
                        selected = self.selected_error_vars[original_index];
                    }
                }
                if ui.checkbox(&mut selected, clean_name).changed() {
                    temp_selections.push((col.clone(), selected));
                }
            }
        });
        
        // Apply checkbox changes
        for (col, selected) in temp_selections {
            if let Some(original_index) = self.error_columns.iter().position(|x| x == &col) {
                if original_index < self.selected_error_vars.len() {
                    self.selected_error_vars[original_index] = selected;
                }
            }
        }
        
        // Plot selected variables
        let selected_columns: Vec<&String> = self.error_columns
            .iter()
            .enumerate()
            .filter_map(|(i, col)| {
                if i < self.selected_error_vars.len() && self.selected_error_vars[i] {
                    Some(col)
                } else {
                    None
                }
            })
            .collect();
        
        if !selected_columns.is_empty() {
            let colors = [
                Color32::RED, Color32::BLUE, Color32::GREEN, Color32::from_rgb(255, 165, 0),
                Color32::from_rgb(128, 0, 128), Color32::from_rgb(165, 42, 42),
                Color32::YELLOW, Color32::from_rgb(255, 192, 203), Color32::DARK_GRAY, Color32::BROWN,
            ];
            
            Plot::new("error_convergence_plot")
                .view_aspect(2.0)
                .height(300.0)
                .legend(egui_plot::Legend::default())
                .show(ui, |plot_ui| {
                    for (plot_idx, col) in selected_columns.iter().enumerate() {
                        let points: PlotPoints = self.records
                            .iter()
                            .filter_map(|r| {
                                r.data.get(*col).map(|&val| [r.iteration as f64, val.abs()])
                            })
                            .collect();
                        
                        let clean_name = col.strip_prefix("Error:").unwrap_or(col);
                        let line = Line::new(points)
                            .color(colors[plot_idx % colors.len()])
                            .width(2.0)
                            .name(clean_name);
                        
                        plot_ui.line(line);
                    }
                });
        }
    }
    
    fn show_value_section(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label(RichText::new("🔵 Value Variables").heading());
        
        let filtered_columns = self.filter_columns(&self.value_columns);
        
        if filtered_columns.is_empty() {
            ui.label("No value variables match the current filter");
            return;
        }
        
        // Variable selection checkboxes
        let mut temp_selections = Vec::new();
        
        ui.horizontal_wrapped(|ui| {
            for col in filtered_columns.iter() {
                let clean_name = col.strip_prefix("Value:").unwrap_or(col);
                let mut selected = false;
                if let Some(original_index) = self.value_columns.iter().position(|x| x == col) {
                    if original_index < self.selected_value_vars.len() {
                        selected = self.selected_value_vars[original_index];
                    }
                }
                if ui.checkbox(&mut selected, clean_name).changed() {
                    temp_selections.push((col.clone(), selected));
                }
            }
        });
        
        // Apply checkbox changes
        for (col, selected) in temp_selections {
            if let Some(original_index) = self.value_columns.iter().position(|x| x == &col) {
                if original_index < self.selected_value_vars.len() {
                    self.selected_value_vars[original_index] = selected;
                }
            }
        }
        
        // Plot selected variables
        let selected_columns: Vec<&String> = self.value_columns
            .iter()
            .enumerate()
            .filter_map(|(i, col)| {
                if i < self.selected_value_vars.len() && self.selected_value_vars[i] {
                    Some(col)
                } else {
                    None
                }
            })
            .collect();
        
        if !selected_columns.is_empty() {
            let colors = [
                Color32::BLUE, Color32::GREEN, Color32::RED, Color32::from_rgb(255, 165, 0),
                Color32::from_rgb(128, 0, 128), Color32::from_rgb(165, 42, 42),
                Color32::YELLOW, Color32::from_rgb(255, 192, 203), Color32::DARK_GRAY, Color32::BROWN,
            ];
            
            Plot::new("value_evolution_plot")
                .view_aspect(2.0)
                .height(300.0)
                .legend(egui_plot::Legend::default())
                .show(ui, |plot_ui| {
                    for (plot_idx, col) in selected_columns.iter().enumerate() {
                        let points: PlotPoints = self.records
                            .iter()
                            .filter_map(|r| {
                                r.data.get(*col).map(|&val| [r.iteration as f64, val])
                            })
                            .collect();
                        
                        let clean_name = col.strip_prefix("Value:").unwrap_or(col);
                        let line = Line::new(points)
                            .color(colors[plot_idx % colors.len()])
                            .width(2.0)
                            .name(clean_name);
                        
                        plot_ui.line(line);
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
            // Set default file path if available
            let mut app = CalibrationApp::default();
            
            // Try to auto-load the calibration file if it exists
            let default_path = "Z:\\CalibrationReport.csv";
            if std::path::Path::new(default_path).exists() {
                app.file_path = default_path.to_string();
                let _ = app.load_file(app.file_path.clone());
            }
            
            Ok(Box::new(app))
        }),
    )
}
