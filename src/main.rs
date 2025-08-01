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
    
    // Plot selection - simplified to just variable selection
    selected_vars: Vec<bool>,
    prev_selected_vars: Vec<bool>, // Track previous selection to detect changes
    filter_text: String,
    focus_filter: bool, // Flag to focus filter input on next frame
    filter_has_focus: bool, // Track if filter currently has focus
    
    // Theme state
    is_dark_mode: Option<bool>, // None = follow system, Some(true) = force dark, Some(false) = force light
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
            prev_selected_vars: Vec::new(),
            filter_text: String::new(),
            focus_filter: false,
            filter_has_focus: false,
            is_dark_mode: None, // Start with system default
        }
    }
}


impl CalibrationApp {
    fn apply_theme(&self, ctx: &egui::Context) {
        match self.is_dark_mode {
            Some(true) => ctx.set_visuals(egui::Visuals::dark()),
            Some(false) => ctx.set_visuals(egui::Visuals::light()),
            None => ctx.set_visuals(egui::Visuals::default()),
        }
    }
    
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
    
    fn save_plot_csv(&self, selected_variables: &[(usize, &String)], plot_type: &str) -> Result<()> {
        let default_filename = format!("{}_plot_data.csv", plot_type.to_lowercase());
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .set_file_name(&default_filename)
            .set_title(format!("Save {plot_type} Plot Data"))
            .save_file()
        {
            let mut writer = csv::Writer::from_path(path)?;
            
            // Write header
            let mut header = vec!["Iteration".to_string()];
            for (_, var_name) in selected_variables {
                if plot_type == "Error" && self.has_error_column(var_name) {
                    header.push(format!("{var_name}_Error"));
                } else if plot_type == "Value" && self.has_value_column(var_name) {
                    header.push(format!("{var_name}_Value"));
                }
            }
            writer.write_record(&header)?;
            
            // Write data
            for record in &self.records {
                let mut row = vec![record.iteration.to_string()];
                for (_, var_name) in selected_variables {
                    if plot_type == "Error" && self.has_error_column(var_name) {
                        if let Some(error_col) = self.get_error_column_name(var_name) {
                            if let Some(&val) = record.data.get(&error_col) {
                                row.push(val.to_string());
                            } else {
                                row.push("".to_string());
                            }
                        }
                    } else if plot_type == "Value" && self.has_value_column(var_name) {
                        if let Some(value_col) = self.get_value_column_name(var_name) {
                            if let Some(&val) = record.data.get(&value_col) {
                                row.push(val.to_string());
                            } else {
                                row.push("".to_string());
                            }
                        }
                    }
                }
                writer.write_record(&row)?;
            }
            
            writer.flush()?;
        }
        Ok(())
    }
    
    fn save_plot_image(&self, selected_variables: &[(usize, &String)], plot_type: &str, colors: &[Color32], plot_bounds: Option<&egui_plot::PlotBounds>, ctx: &egui::Context) -> Result<()> {
        let default_filename = format!("{}_plot.png", plot_type.to_lowercase());
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG Images", &["png"])
            .set_file_name(&default_filename)
            .set_title(format!("Save {plot_type} Plot Image"))
            .save_file()
        {
            use plotters::prelude::*;
            
            // Detect current theme from egui context
            let is_dark_mode = ctx.style().visuals.dark_mode;
            let bg_color = if is_dark_mode {
                RGBColor(32, 32, 32) // Dark background
            } else {
                WHITE // Light background
            };
            let text_color = if is_dark_mode {
                RGBColor(255, 255, 255) // White text for dark mode
            } else {
                RGBColor(0, 0, 0) // Black text for light mode
            };
            let grid_color = if is_dark_mode {
                RGBColor(64, 64, 64) // Light gray grid lines for dark mode
            } else {
                RGBColor(128, 128, 128) // Dark gray grid lines for light mode
            };
            
            let root = BitMapBackend::new(&path, (1600, 1200)).into_drawing_area();
            root.fill(&bg_color)?;
            
            // Use plot bounds if provided, otherwise calculate from data
            let (x_range, y_range) = if let Some(bounds) = plot_bounds {
                let x_min = bounds.min()[0];
                let x_max = bounds.max()[0];
                let y_min = bounds.min()[1];
                let y_max = bounds.max()[1];
                (x_min..x_max, y_min..y_max)
            } else {
                // Fallback to calculating from all data
                let x_range = 0f64..self.records.len() as f64;
                let y_range = {
                    let mut min_val = f64::INFINITY;
                    let mut max_val = f64::NEG_INFINITY;
                    
                    for (_, var_name) in selected_variables {
                        if plot_type == "Error" && self.has_error_column(var_name) {
                            if let Some(error_col) = self.get_error_column_name(var_name) {
                                for record in &self.records {
                                    if let Some(&val) = record.data.get(&error_col) {
                                        min_val = min_val.min(val);
                                        max_val = max_val.max(val);
                                    }
                                }
                            }
                        } else if plot_type == "Value" && self.has_value_column(var_name) {
                            if let Some(value_col) = self.get_value_column_name(var_name) {
                                for record in &self.records {
                                    if let Some(&val) = record.data.get(&value_col) {
                                        min_val = min_val.min(val);
                                        max_val = max_val.max(val);
                                    }
                                }
                            }
                        }
                    }
                    
                    let range = max_val - min_val;
                    let margin = range * 0.1;
                    (min_val - margin)..(max_val + margin)
                };
                (x_range, y_range)
            };
            
            let mut chart = ChartBuilder::on(&root)
                .caption(format!("{plot_type} Convergence"), ("Arial", 60).into_font().color(&text_color))
                .margin(40)
                .x_label_area_size(100)
                .y_label_area_size(160)
                .build_cartesian_2d(x_range, y_range)?;
            
            chart
                .configure_mesh()
                .x_desc("Iteration")
                .y_desc(if plot_type == "Error" { "Absolute Error" } else { "Value" })
                .axis_desc_style(("Arial", 30).into_font().color(&text_color))
                .label_style(("Arial", 24).into_font().color(&text_color))
                .axis_style(text_color)
                .light_line_style(grid_color)
                .bold_line_style(grid_color)
                .draw()?;
            
            let mut plot_idx = 0;
            for (_, var_name) in selected_variables {
                if plot_type == "Error" && self.has_error_column(var_name) {
                    if let Some(error_col) = self.get_error_column_name(var_name) {
                        let points: Vec<(f64, f64)> = self.records
                            .iter()
                            .filter_map(|r| {
                                r.data.get(&error_col).map(|&val| (r.iteration as f64, val))
                            })
                            .collect();
                        
                        let color = colors[plot_idx % colors.len()];
                        let rgb_color = RGBColor(color.r(), color.g(), color.b());
                        
                        chart.draw_series(LineSeries::new(points, &rgb_color))?
                            .label(*var_name)
                            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], rgb_color));
                        
                        plot_idx += 1;
                    }
                } else if plot_type == "Value" && self.has_value_column(var_name) {
                    if let Some(value_col) = self.get_value_column_name(var_name) {
                        let points: Vec<(f64, f64)> = self.records
                            .iter()
                            .filter_map(|r| {
                                r.data.get(&value_col).map(|&val| (r.iteration as f64, val))
                            })
                            .collect();
                        
                        let color = colors[plot_idx % colors.len()];
                        let rgb_color = RGBColor(color.r(), color.g(), color.b());
                        
                        chart.draw_series(LineSeries::new(points, &rgb_color))?
                            .label(*var_name)
                            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], rgb_color));
                        
                        plot_idx += 1;
                    }
                }
            }
            
            chart.configure_series_labels()
                .background_style(bg_color.mix(0.8))
                .border_style(text_color)
                .label_font(("Arial", 24).into_font().color(&text_color))
                .position(plotters::chart::SeriesLabelPosition::UpperRight)
                .margin(20)
                .draw()?;
            root.present()?;
        }
        Ok(())
    }
}

impl eframe::App for CalibrationApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme at the beginning of each frame
        self.apply_theme(ctx);
        
        // Handle keyboard shortcuts
        ctx.input(|i| {
            // Ctrl+F to focus filter
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) {
                self.focus_filter = true;
            }
            
            // Escape to clear filter when filter has focus
            if i.key_pressed(egui::Key::Escape) && self.filter_has_focus {
                self.filter_text.clear();
            }
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with title and theme toggle
            ui.horizontal(|ui| {
                ui.heading("📊 Calibration Report Visualizer");
                
                // Push the theme toggle to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Theme toggle button
                    let theme_text = match self.is_dark_mode {
                        Some(true) => "🌙 Dark",
                        Some(false) => "💡 Light", 
                        None => "🔄 System",
                    };
                    
                    let theme_button = ui.button(theme_text);
                    if theme_button.clicked() {
                        self.is_dark_mode = match self.is_dark_mode {
                            None => Some(true),        // System -> Dark
                            Some(true) => Some(false), // Dark -> Light
                            Some(false) => None,       // Light -> System
                        };
                    }
                    theme_button.on_hover_text("Click to cycle between System, Dark, and Light themes");
                });
            });
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
                
                let filter_response = ui.text_edit_singleline(&mut self.filter_text);
                
                // Track filter focus state
                self.filter_has_focus = filter_response.has_focus();
                
                // Handle focus request from keyboard shortcut
                if self.focus_filter {
                    filter_response.request_focus();
                    self.focus_filter = false;
                }
                
                // Add tooltip with more information
                filter_response.on_hover_text("Filter variables by name. Use commas to separate multiple terms. Press Ctrl+F to focus this field, Esc to clear when focused.");
                
                // Add hint about keyboard shortcuts
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small("(Ctrl+F, Esc to clear)");
                });
            });
            
            ui.separator();
            
            // Variable selection and plotting
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_variables_section(ui, ctx);
            });
        });
    }
}

impl CalibrationApp {
    fn show_variables_section(&mut self, ui: &mut Ui, ctx: &egui::Context) {
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
            ui.label(format!("📊 Showing {total_vars} variables"));
        });
        
        ui.separator();
        
        // Get selected variables and create color mapping
        let selected_variables: Vec<(usize, &String)> = self.variable_names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < self.selected_vars.len() && self.selected_vars[*i])
            .collect();
        
        let colors = [
            Color32::RED, Color32::BLUE, Color32::GREEN, Color32::from_rgb(255, 165, 0),
            Color32::from_rgb(128, 0, 128), Color32::from_rgb(165, 42, 42),
            Color32::YELLOW, Color32::from_rgb(255, 192, 203), Color32::DARK_GRAY, Color32::BROWN,
        ];
        
        // Create a mapping from variable name to color index for selected variables
        let mut variable_color_map = std::collections::HashMap::new();
        let mut color_idx = 0;
        for (_, var_name) in &selected_variables {
            if self.has_error_column(var_name) || self.has_value_column(var_name) {
                variable_color_map.insert(var_name.as_str(), color_idx % colors.len());
                color_idx += 1;
            }
        }
        // Variable selection and options in scrollable area
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                // Calculate optimal number of columns based on available width
                // Estimate column width: checkbox + text + padding (~200px per column)
                let available_width = ui.available_width();
                let estimated_column_width = 200.0;
                let columns_count = (available_width / estimated_column_width) as usize;
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
                                            
                                            // Style the checkbox based on selection and color mapping
                                            if selected {
                                                if let Some(&color_index) = variable_color_map.get(var_name.as_str()) {
                                                    let graph_color = colors[color_index];
                                                    
                                                    // Create a custom checkbox style with the graph color
                                                    let mut checkbox_style = ui.style().visuals.widgets.inactive;
                                                    checkbox_style.bg_fill = graph_color;
                                                    checkbox_style.bg_stroke = egui::Stroke::new(1.0, graph_color.gamma_multiply(0.8));
                                                    
                                                    let mut active_style = ui.style().visuals.widgets.active;
                                                    active_style.bg_fill = graph_color;
                                                    active_style.bg_stroke = egui::Stroke::new(2.0, graph_color.gamma_multiply(0.8));
                                                    
                                                    ui.style_mut().visuals.widgets.inactive = checkbox_style;
                                                    ui.style_mut().visuals.widgets.active = active_style;
                                                }
                                            }
                                            
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
        // Check if selection has changed to reset view
        let selection_changed = self.selected_vars != self.prev_selected_vars;
        if selection_changed {
            self.prev_selected_vars = self.selected_vars.clone();
        }
        
        if !selected_variables.is_empty() {
            ui.separator();
            ui.label(RichText::new("📈 Selected Variables Plots").heading());
            ui.separator();
            
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
                        ui.label(RichText::new("🔴 Error").strong());
                        ui.add_space(2.0); // Increased spacing after label
                        
                        let mut error_plot = Plot::new("error_plot")
                            .view_aspect(2.0) // Increased aspect ratio for more horizontal space
                            .height(450.0) // Increased height
                            .width(plot_width) // Reduced width to add margins
                            .legend(egui_plot::Legend::default())
                            .x_axis_label("Iteration")
                            .y_axis_label("Error");
                        
                        // Reset view if selection changed
                        if selection_changed {
                            error_plot = error_plot.auto_bounds(egui::Vec2b::new(true, true)).reset();
                        }
                        
                        let error_plot_response = error_plot.show(ui, |plot_ui| {
                                let mut plot_idx = 0;
                                
                                for (_, var_name) in &selected_variables {
                                    if self.has_error_column(var_name) {
                                        if let Some(error_col) = self.get_error_column_name(var_name) {
                                            let points: PlotPoints = self.records
                                                .iter()
                                                .filter_map(|r| {
                                                    r.data.get(&error_col).map(|&val| [r.iteration as f64, val])
                                                })
                                                .collect();
                                            
                                            let line = Line::new(var_name.as_str(), points)
                                                .color(colors[plot_idx % colors.len()])
                                                .width(2.0);
                                            
                                            plot_ui.line(line);
                                            plot_idx += 1;
                                        }
                                    }
                                }
                            });
                        
                        // Handle right-click context menu for error plot
                        error_plot_response.response.context_menu(|ui| {
                            if ui.button("💾 Save as CSV").clicked() {
                                if let Err(e) = self.save_plot_csv(&selected_variables, "Error") {
                                    eprintln!("Failed to save CSV: {e}");
                                }
                                ui.close();
                            }
                            if ui.button("📸 Save as Image").clicked() {
                                if let Err(e) = self.save_plot_image(&selected_variables, "Error", &colors, Some(error_plot_response.transform.bounds()), ctx) {
                                    eprintln!("Failed to save image: {e}");
                                }
                                ui.close();
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
                        ui.label(RichText::new("🔵 Value").strong());
                        ui.add_space(2.0); // Increased spacing after label
                        
                        let mut value_plot = Plot::new("value_plot")
                            .view_aspect(2.0) // Increased aspect ratio for more horizontal space
                            .height(450.0) // Increased height
                            .width(plot_width) // Reduced width to add margins
                            .legend(egui_plot::Legend::default())
                            .x_axis_label("Iteration")
                            .y_axis_label("Value");
                        
                        // Reset view if selection changed
                        if selection_changed {
                            value_plot = value_plot.auto_bounds(egui::Vec2b::new(true, true)).reset();
                        }
                        
                        let value_plot_response = value_plot.show(ui, |plot_ui| {
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
                                            
                                            let line = Line::new(var_name.as_str(), points)
                                                .color(colors[plot_idx % colors.len()])
                                                .width(2.0);
                                            
                                            plot_ui.line(line);
                                            plot_idx += 1;
                                        }
                                    }
                                }
                            });
                        
                        // Handle right-click context menu for value plot
                        value_plot_response.response.context_menu(|ui| {
                            if ui.button("💾 Save as CSV").clicked() {
                                if let Err(e) = self.save_plot_csv(&selected_variables, "Value") {
                                    eprintln!("Failed to save CSV: {e}");
                                }
                                ui.close();
                            }
                            if ui.button("📸 Save as Image").clicked() {
                                if let Err(e) = self.save_plot_image(&selected_variables, "Value", &colors, Some(value_plot_response.transform.bounds()), ctx) {
                                    eprintln!("Failed to save image: {e}");
                                }
                                ui.close();
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
