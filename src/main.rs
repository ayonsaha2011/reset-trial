use eframe::{egui, epaint::ImageData, IconData};
use serde::{Deserialize, Serialize};
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;
use image::ImageFormat;
use std::fs::File;
use std::io::Cursor;

const HISTORY_FILE: &str = "search_history.json";

#[derive(Serialize, Deserialize, Default)]
struct ResetTrialApp {
    search_keyword: String,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    logs: Vec<String>,
    search_history: Vec<String>,
    confirmation_pending: Option<(PathBuf, bool)>, // File or directory awaiting delete confirmation
    confirm_delete_all: bool,                     // Flag for delete-all confirmation
    pending_history_removal: Option<usize>,       // Search history item to delete
}

impl ResetTrialApp {
    fn new() -> Self {
        let mut app = Self::default();
        app.ensure_history_file();
        app.load_history();
        app.logs.push("Welcome to Reset Trial App!".to_string());
        app
    }

    fn search_files(&mut self) {
        self.files.clear();
        self.directories.clear();
        self.logs.clear();

        if self.search_keyword.is_empty() {
            self.logs.push("Please enter a valid search keyword.".to_string());
            return;
        }

        if !self.search_history.contains(&self.search_keyword) {
            self.search_history.push(self.search_keyword.clone());
            self.save_history();
        }

        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        for entry in WalkDir::new(&home_dir) {
            if let Ok(entry) = entry {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                if file_name.contains(&self.search_keyword.to_lowercase()) {
                    if entry.file_type().is_file() {
                        self.files.push(entry.into_path());
                    } else if entry.file_type().is_dir() {
                        self.directories.push(entry.into_path());
                    }
                }
            }
        }

        if self.files.is_empty() && self.directories.is_empty() {
            self.logs.push("No matching files or directories found.".to_string());
        } else {
            self.logs.push(format!(
                "Found {} files and {} directories.",
                self.files.len(),
                self.directories.len()
            ));
        }
    }

    fn delete_item(&mut self, path: PathBuf, is_file: bool) {
        let result = if is_file {
            fs::remove_file(&path)
        } else {
            fs::remove_dir_all(&path)
        };

        if let Err(e) = result {
            self.logs.push(format!("Failed to delete {:?}: {}", path, e));
        } else {
            self.logs.push(format!("Deleted: {:?}", path));
        }
    }

    fn delete_all(&mut self) {
        for file in self.files.clone() {
            self.delete_item(file, true);
        }
        for dir in self.directories.clone() {
            self.delete_item(dir, false);
        }
        self.files.clear();
        self.directories.clear();
        self.logs.push("Deleted all files and directories.".to_string());
    }

    fn ensure_history_file(&self) {
        if !Path::new(HISTORY_FILE).exists() {
            if let Err(e) = fs::write(HISTORY_FILE, "[]") {
                eprintln!("Failed to create history file: {}", e);
            }
        }
    }

    fn save_history(&self) {
        if let Ok(json) = serde_json::to_string(&self.search_history) {
            if let Err(e) = fs::write(HISTORY_FILE, json) {
                eprintln!("Failed to save search history: {}", e);
            }
        }
    }

    fn load_history(&mut self) {
        if let Ok(json) = fs::read_to_string(HISTORY_FILE) {
            if let Ok(history) = serde_json::from_str::<Vec<String>>(&json) {
                self.search_history = history;
            }
        }
    }

    fn list_directory_contents(&self, dir: &PathBuf) -> Vec<(PathBuf, bool)> {
        let mut contents = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    contents.push((entry.path(), is_dir));
                }
            }
        }
        contents
    }
}

impl eframe::App for ResetTrialApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(18.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(18.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(14.0, egui::FontFamily::Proportional))
        ]
            .into();
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Reset Trial App");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("🔍 Search Keyword:");
                    ui.text_edit_singleline(&mut self.search_keyword);
                    if ui.button("Search").clicked() {
                        self.search_files();
                    }
                });

                ui.add_space(10.0);

                if !self.search_history.is_empty() {
                    ui.collapsing("📜 Search History", |ui| {
                        let search_history = self.search_history.clone();
                        for (index, keyword) in search_history.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}: {}", index + 1, keyword));
                                if ui.button("Use").clicked() {
                                    self.search_keyword = keyword.clone();
                                    self.search_files();
                                }
                                if ui.button("Delete").clicked() {
                                    self.pending_history_removal = Some(index);
                                }
                            });
                            ui.add_space(5.0);
                        }
                    });
                    ui.add_space(10.0);
                }

                if !self.files.is_empty() || !self.directories.is_empty() {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("🗂️ Files and Directories");
                            ui.add_space(10.0);

                            ui.collapsing("📂 Files", |ui| {
                                for (index, file) in self.files.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}: {:?}", index + 1, file));
                                        if ui.button("Delete").clicked() {
                                            self.confirmation_pending = Some((file.clone(), true));
                                        }
                                    });
                                    ui.add_space(5.0);
                                }
                            });

                            ui.add_space(10.0);

                            ui.collapsing("📁 Directories", |ui| {
                                for (index, dir) in self.directories.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}: {:?}", index + 1, dir));
                                        if ui.button("Delete").clicked() {
                                            self.confirmation_pending = Some((dir.clone(), false));
                                        }
                                    });

                                    for (content_path, is_dir) in self.list_directory_contents(dir) {
                                        ui.horizontal(|ui| {
                                            ui.label(format!(
                                                "    {}: {:?}",
                                                if is_dir { "📁" } else { "📄" },
                                                content_path.file_name().unwrap_or_default()
                                            ));
                                            if ui.button("Delete").clicked() {
                                                self.confirmation_pending =
                                                    Some((content_path.clone(), !is_dir));
                                            }
                                        });
                                    }

                                    ui.add_space(5.0);
                                }
                            });

                            ui.add_space(10.0);

                            if ui.button("🗑️ Delete All").clicked() {
                                self.confirm_delete_all = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                } else {
                    ui.label("No files or directories to display.");
                    ui.add_space(10.0);
                }

                ui.collapsing("📋 Logs", |ui| {
                    for log in &self.logs {
                        ui.label(egui::RichText::new(log).text_style(egui::TextStyle::Small));
                        ui.add_space(5.0);
                    }
                });
            });
        });

        if let Some((path, is_file)) = &self.confirmation_pending.clone() {
            let path_clone = path.clone();
            let is_file_clone = *is_file;

            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Are you sure you want to delete this {}?",
                        if is_file_clone { "file" } else { "directory" }
                    ));
                    ui.label(format!("{:?}", path_clone));
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.delete_item(path_clone, is_file_clone);
                            self.confirmation_pending = None;
                        }
                        if ui.button("No").clicked() {
                            self.confirmation_pending = None;
                        }
                    });
                });
        }

        if self.confirm_delete_all {
            egui::Window::new("Confirm Delete All")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to delete all files and directories?");
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.delete_all();
                            self.confirm_delete_all = false;
                        }
                        if ui.button("No").clicked() {
                            self.confirm_delete_all = false;
                        }
                    });
                });
        }

        if let Some(index) = self.pending_history_removal {
            let history_item = self.search_history.get(index).cloned();
            if let Some(item) = history_item {
                egui::Window::new("Confirm Search History Deletion")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Are you sure you want to delete this search history?");
                        ui.label(item);
                        ui.horizontal(|ui| {
                            if ui.button("Yes").clicked() {
                                self.search_history.remove(index);
                                self.pending_history_removal = None;
                                self.save_history();
                            }
                            if ui.button("No").clicked() {
                                self.pending_history_removal = None;
                            }
                        });
                    });
            }
        }
    }
}

fn load_icon() -> Option<IconData> {
    // Resolve the bundled resource path
    let mut resource_path = std::env::current_exe().ok()?;
    resource_path.pop(); // Remove executable name
    resource_path.pop(); // Remove 'MacOS'
    resource_path.push("Resources/assets/logo.png"); // Add the icon path

    // Load the icon
    let icon_data = std::fs::read(resource_path).ok()?;
    if let Ok(img) = image::load(Cursor::new(icon_data), ImageFormat::Png) {
        let img = img.to_rgba8();
        let (width, height) = img.dimensions();
        let rgba = img.as_raw().clone();
        Some(IconData {
            rgba,
            width,
            height,
        })
    } else {
        None
    }
}
fn main() -> Result<(), eframe::Error> {
    let mut options = eframe::NativeOptions::default();

    // Set app icon
    if let Some(icon_data) = load_icon() {
        options.icon_data = Some(icon_data);
    }

    options.resizable = true;

    eframe::run_native(
        "Reset Trial App",
        options,
        Box::new(|_cc| Box::new(ResetTrialApp::new())),
    )
}
