use eframe::egui;
use egui::IconData;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use image::ImageFormat;
use std::io::Cursor;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

const HISTORY_FILE: &str = "search_history.json";

enum SearchMessage {
    FoundFile(PathBuf),
    FoundDirectory(PathBuf),
    SearchFinished,
    SearchError(String),
}

#[derive(Serialize, Deserialize, Default)]
struct AppState {
    search_keyword: String,
    filter_query: String,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    logs: Vec<String>,
    search_history: Vec<String>,
}

struct ResetTrialApp {
    state: AppState,
    is_searching: bool,
    search_rx: Option<Receiver<SearchMessage>>,
    confirmation_pending: Option<(PathBuf, bool)>,
    confirm_delete_all: bool,
    pending_history_removal: Option<usize>,
    sidebar_visible: bool,
}

impl ResetTrialApp {
    fn new() -> Self {
        let mut app = Self {
            state: AppState::default(),
            is_searching: false,
            search_rx: None,
            confirmation_pending: None,
            confirm_delete_all: false,
            pending_history_removal: None,
            sidebar_visible: true,
        };
        app.ensure_history_file();
        app.load_history();
        app.state.logs.push("Welcome to Reset Trial App!".to_string());
        app
    }

    fn search_files(&mut self) {
        if self.state.search_keyword.is_empty() {
            self.state.logs.push("Please enter a valid search keyword.".to_string());
            return;
        }

        self.state.files.clear();
        self.state.directories.clear();
        self.state.logs.clear();
        self.is_searching = true;

        if !self.state.search_history.contains(&self.state.search_keyword) {
            self.state.search_history.push(self.state.search_keyword.clone());
            self.save_history();
        }

        let (tx, rx) = channel();
        self.search_rx = Some(rx);
        let keyword = self.state.search_keyword.clone().to_lowercase();

        thread::spawn(move || {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            for entry in WalkDir::new(&home_dir) {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name().to_string_lossy().to_lowercase();
                    if file_name.contains(&keyword) {
                        if entry.file_type().is_file() {
                            let _ = tx.send(SearchMessage::FoundFile(entry.into_path()));
                        } else if entry.file_type().is_dir() {
                            let _ = tx.send(SearchMessage::FoundDirectory(entry.into_path()));
                        }
                    }
                }
            }
            let _ = tx.send(SearchMessage::SearchFinished);
        });
    }

    fn handle_search_messages(&mut self) {
        if let Some(rx) = &self.search_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    SearchMessage::FoundFile(path) => self.state.files.push(path),
                    SearchMessage::FoundDirectory(path) => self.state.directories.push(path),
                    SearchMessage::SearchFinished => {
                        self.is_searching = false;
                        self.state.logs.push(format!(
                            "Search finished. Found {} files and {} directories.",
                            self.state.files.len(),
                            self.state.directories.len()
                        ));
                    }
                    SearchMessage::SearchError(e) => {
                        self.is_searching = false;
                        self.state.logs.push(format!("Search error: {}", e));
                    }
                }
            }
        }
    }

    fn delete_item(&mut self, path: PathBuf, is_file: bool) {
        let result = if is_file {
            fs::remove_file(&path)
        } else {
            fs::remove_dir_all(&path)
        };

        if let Err(e) = result {
            self.state.logs.push(format!("Failed to delete {:?}: {}", path, e));
        } else {
            self.state.logs.push(format!("Deleted: {:?}", path));
            // Remove from results if it was there
            if is_file {
                self.state.files.retain(|f| f != &path);
            } else {
                self.state.directories.retain(|d| d != &path);
            }
        }
    }

    fn delete_all(&mut self) {
        for file in self.state.files.clone() {
            self.delete_item(file, true);
        }
        for dir in self.state.directories.clone() {
            self.delete_item(dir, false);
        }
        self.state.files.clear();
        self.state.directories.clear();
        self.state.logs.push("Deleted all files and directories.".to_string());
    }

    fn ensure_history_file(&self) {
        if !Path::new(HISTORY_FILE).exists() {
            if let Err(e) = fs::write(HISTORY_FILE, "[]") {
                eprintln!("Failed to create history file: {}", e);
            }
        }
    }

    fn save_history(&self) {
        if let Ok(json) = serde_json::to_string(&self.state.search_history) {
            if let Err(e) = fs::write(HISTORY_FILE, json) {
                eprintln!("Failed to save search history: {}", e);
            }
        }
    }

    fn load_history(&mut self) {
        if let Ok(json) = fs::read_to_string(HISTORY_FILE) {
            if let Ok(history) = serde_json::from_str::<Vec<String>>(&json) {
                self.state.search_history = history;
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

    fn open_in_explorer(path: &Path) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg("/select,")
                .arg(path)
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(parent) = path.parent() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(parent)
                    .spawn();
            }
        }
    }
}

impl eframe::App for ResetTrialApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_search_messages();
        ctx.request_repaint(); // Keep UI updated during async search

        // Custom Styling (Modern / Glassmorphic)
        let mut visuals = egui::Visuals::dark();
        let bg_color = egui::Color32::from_rgb(10, 12, 18);
        let panel_color = egui::Color32::from_rgba_premultiplied(20, 25, 35, 200); // Semi-transparent
        let accent_color = egui::Color32::from_rgb(0, 180, 255);
        
        visuals.widgets.noninteractive.bg_fill = bg_color;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 20));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(40, 45, 60, 150);
        visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(60, 70, 90, 200);
        visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
        visuals.widgets.active.bg_fill = accent_color;
        visuals.widgets.active.rounding = egui::Rounding::same(8.0);
        
        visuals.selection.bg_fill = accent_color.linear_multiply(0.5);
        visuals.window_fill = bg_color;
        visuals.window_rounding = egui::Rounding::same(12.0);
        visuals.panel_fill = panel_color;
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(32.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Small, egui::FontId::new(13.0, egui::FontFamily::Proportional)),
        ].into();
        style.spacing.item_spacing = egui::vec2(12.0, 12.0);
        style.spacing.window_margin = egui::Margin::same(20.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.visuals.window_shadow = egui::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 30.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(100),
        };
        ctx.set_style(style);

        // Sidebar
        if self.sidebar_visible {
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(260.0)
                .show(ctx, |ui| {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("🚀 Reset Trial");
                    });
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new("📜 History").strong());
                    egui::ScrollArea::vertical()
                        .id_salt("history_scroll")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let history = self.state.search_history.clone();
                            for (index, keyword) in history.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.button(format!("🔍 {}", keyword)).clicked() {
                                        self.state.search_keyword = keyword.clone();
                                        self.search_files();
                                    }
                                    if ui.button("🗑").clicked() {
                                        self.pending_history_removal = Some(index);
                                    }
                                });
                            }
                        });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new("📋 Activity Logs").strong());
                    egui::ScrollArea::vertical()
                        .id_salt("logs_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for log in &self.state.logs {
                                ui.label(egui::RichText::new(log).text_style(egui::TextStyle::Small).color(egui::Color32::from_gray(160)));
                            }
                        });
                });
        }

        // Top Panel for Search
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(if self.sidebar_visible { "⬅" } else { "➡" }).clicked() {
                    self.sidebar_visible = !self.sidebar_visible;
                }

                ui.add_space(10.0);
                ui.label("Search:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.state.search_keyword)
                        .hint_text("Enter software name...")
                        .desired_width(f32::INFINITY)
                );
                
                if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Search").clicked() {
                    self.search_files();
                }

                if self.is_searching {
                    ui.add(egui::Spinner::new());
                }
            });
            ui.add_space(10.0);
        });

        // Main Content
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.files.is_empty() && self.state.directories.is_empty() && !self.is_searching {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("✨ Ready to clean").size(32.0).color(egui::Color32::from_gray(120)));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Enter a software name to find associated trial files.").color(egui::Color32::from_gray(100)));
                    
                    if ui.button("Example: Adobe").clicked() {
                        self.state.search_keyword = "Adobe".to_string();
                        self.search_files();
                    }
                });
            } else {
                ui.vertical(|ui| {
                    // Filter Bar
                    ui.horizontal(|ui| {
                        ui.label("🔍 Filter:");
                        ui.add(egui::TextEdit::singleline(&mut self.state.filter_query).hint_text("Filter by name...").desired_width(200.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.state.files.is_empty() || !self.state.directories.is_empty() {
                                ui.label(format!("Found {} items", self.state.files.len() + self.state.directories.len()));
                            }
                        });
                    });
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                        let filter = self.state.filter_query.to_lowercase();
                        
                        if !self.state.files.is_empty() {
                            ui.heading("📄 Files");
                            ui.add_space(5.0);
                            for file in self.state.files.clone() {
                                let filename = file.file_name().unwrap_or_default().to_string_lossy();
                                if filename.to_lowercase().contains(&filter) {
                                    ui.add_space(4.0);
                                    ui.scope(|ui| {
                                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 5);
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(filename).strong().color(egui::Color32::from_rgb(200, 220, 255)));
                                                    ui.label(egui::RichText::new(format!("{:?}", file)).small().color(egui::Color32::from_gray(120)));
                                                });
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button("🗑").on_hover_text("Delete this file").clicked() {
                                                        self.confirmation_pending = Some((file.clone(), true));
                                                    }
                                                    if ui.button("📂").on_hover_text("Open in Explorer").clicked() {
                                                        Self::open_in_explorer(&file);
                                                    }
                                                });
                                            });
                                        });
                                    });
                                }
                            }
                            ui.add_space(15.0);
                        }

                        if !self.state.directories.is_empty() {
                            ui.heading("📁 Directories");
                            ui.add_space(5.0);
                            for dir in self.state.directories.clone() {
                                let dirname = dir.file_name().unwrap_or_default().to_string_lossy();
                                if dirname.to_lowercase().contains(&filter) {
                                    ui.add_space(4.0);
                                    ui.scope(|ui| {
                                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(0, 150, 255, 10);
                                        ui.group(|ui| {
                                            ui.collapsing(egui::RichText::new(dirname).strong().color(egui::Color32::from_rgb(180, 255, 180)), |ui| {
                                                for (content_path, is_dir) in self.list_directory_contents(&dir) {
                                                    ui.horizontal(|ui| {
                                                        ui.label(format!("  {} {}", if is_dir { "📁" } else { "📄" }, content_path.file_name().unwrap_or_default().to_string_lossy()));
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.button("🗑").clicked() {
                                                                self.confirmation_pending = Some((content_path.clone(), !is_dir));
                                                            }
                                                            if ui.button("📂").clicked() {
                                                                Self::open_in_explorer(&content_path);
                                                            }
                                                        });
                                                    });
                                                }
                                            });
                                            
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.add(egui::Button::new("🗑 Delete Folder").fill(egui::Color32::from_rgb(120, 40, 40))).clicked() {
                                                    self.confirmation_pending = Some((dir.clone(), false));
                                                }
                                                if ui.button("📂 Show").clicked() {
                                                    Self::open_in_explorer(&dir);
                                                }
                                            });
                                        });
                                    });
                                }
                            }
                        }

                        if !self.state.files.is_empty() || !self.state.directories.is_empty() {
                            ui.add_space(30.0);
                            ui.vertical_centered(|ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🗑 DELETE ALL FOUND ITEMS").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(200, 50, 50))).clicked() {
                                    self.confirm_delete_all = true;
                                }
                            });
                            ui.add_space(20.0);
                        }
                    });
                });
            }
        });

        // Dialogs
        if let Some((path, is_file)) = &self.confirmation_pending.clone() {
            egui::Window::new("⚠️ Confirm Deletion")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Delete this {}?", if *is_file { "file" } else { "directory" }));
                    ui.label(egui::RichText::new(format!("{:?}", path)).small());
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Confirm").clicked() {
                            self.delete_item(path.clone(), *is_file);
                            self.confirmation_pending = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirmation_pending = None;
                        }
                    });
                });
        }

        if self.confirm_delete_all {
            egui::Window::new("🚨 Confirm Delete All")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to delete ALL found items? This cannot be undone.");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("YES, DELETE ALL").clicked() {
                            self.delete_all();
                            self.confirm_delete_all = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete_all = false;
                        }
                    });
                });
        }

        if let Some(index) = self.pending_history_removal {
            if let Some(item) = self.state.search_history.get(index).cloned() {
                egui::Window::new("Clear History")
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!("Remove '{}' from history?", item));
                        ui.horizontal(|ui| {
                            if ui.button("Remove").clicked() {
                                self.state.search_history.remove(index);
                                self.pending_history_removal = None;
                                self.save_history();
                            }
                            if ui.button("Cancel").clicked() {
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

    if let Some(icon_data) = load_icon() {
        options.viewport = options.viewport.with_icon(icon_data);
    }

    eframe::run_native(
        "Reset Trial App",
        options,
        Box::new(|_cc| Ok(Box::new(ResetTrialApp::new()))),
    )
}
