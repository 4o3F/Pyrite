use crate::models;
use crate::services::config_loader::{self, PyriteConfig};
use crate::services::event_parser::{ParserEvent, spawn_event_feed_parser};
use eframe::egui;
use rfd::FileDialog;
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Mutex, OnceLock};

pub enum LoadDataAction {
    Stay,
    Continue,
}

#[derive(Default)]
struct ParseUiState {
    receiver: Option<Receiver<ParserEvent>>,
    is_parsing: bool,
    parsed_successfully: bool,
    parsed_path: Option<String>,
    lines_read: u64,
    error_count: u64,
    parse_failed_message: Option<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    warnings_acknowledged: bool,
    parsed_contest_state: Option<models::ContestState>,
    parsed_config: Option<PyriteConfig>,
}

static PARSE_STATE: OnceLock<Mutex<ParseUiState>> = OnceLock::new();

fn parse_state() -> &'static Mutex<ParseUiState> {
    PARSE_STATE.get_or_init(|| Mutex::new(ParseUiState::default()))
}

fn validate_cdp_folder(folder_path: &str) -> Result<String, Vec<String>> {
    let mut errors = Vec::new();
    let folder = Path::new(folder_path);

    if !folder.exists() {
        errors.push(format!("CDP folder does not exist: {}", folder.display()));
        return Err(errors);
    }

    if !folder.is_dir() {
        errors.push(format!("Path is not a folder: {}", folder.display()));
        return Err(errors);
    }

    let teams_dir = folder.join("teams");
    if !teams_dir.is_dir() {
        errors.push(format!("Missing required folder: {}", teams_dir.display()));
    }

    let affiliations_dir = folder.join("affiliations");
    if !affiliations_dir.is_dir() {
        errors.push(format!(
            "Missing required folder: {}",
            affiliations_dir.display()
        ));
    }

    let event_feed = folder.join("event-feed.ndjson");
    if !event_feed.is_file() {
        errors.push(format!("Missing required file: {}", event_feed.display()));
    }

    let config_toml = folder.join("config.toml");
    if config_toml.exists() && !config_toml.is_file() {
        errors.push(format!(
            "config.toml exists but is not a file: {}",
            config_toml.display()
        ));
    }

    if errors.is_empty() {
        Ok(event_feed.display().to_string())
    } else {
        Err(errors)
    }
}

pub fn take_parsed_contest_state() -> Option<models::ContestState> {
    let mut state = parse_state().lock().expect("parse state lock poisoned");
    state.parsed_contest_state.take()
}

pub fn take_parsed_config() -> Option<PyriteConfig> {
    let mut state = parse_state().lock().expect("parse state lock poisoned");
    state.parsed_config.take()
}

pub fn ui(ui: &mut egui::Ui, data_path: &mut Option<String>) -> LoadDataAction {
    ui.heading("Pyrite");
    ui.add_space(8.0);
    ui.label("Select CDP folder path");
    ui.add_space(12.0);

    ui.label("CDP folder:");
    let mut selected_path = data_path.clone().unwrap_or_default();
    let response = ui.add_sized(
        [900.0, 28.0],
        egui::TextEdit::singleline(&mut selected_path).hint_text("Enter CDP folder path..."),
    );
    if response.changed() {
        let trimmed = selected_path.trim().to_string();
        if trimmed.is_empty() {
            *data_path = None;
        } else {
            *data_path = Some(trimmed);
        }
    }
    ui.add_space(8.0);

    if ui.button("Choose folder").clicked()
        && let Some(path) = FileDialog::new().set_directory(".").pick_folder()
    {
        *data_path = Some(path.display().to_string());
    }

    let current_path = data_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);

    let mut state = parse_state().lock().expect("parse state lock poisoned");

    if current_path != state.parsed_path && !state.is_parsing {
        state.parsed_successfully = false;
        state.lines_read = 0;
        state.error_count = 0;
        state.parse_failed_message = None;
        state.errors.clear();
        state.warnings.clear();
        state.warnings_acknowledged = false;
        state.receiver = None;
        state.parsed_contest_state = None;
        state.parsed_config = None;
    }

    if state.is_parsing {
        loop {
            let event = {
                let Some(rx) = &state.receiver else {
                    break;
                };
                rx.try_recv()
            };

            match event {
                Ok(ParserEvent::Started) => {
                    state.is_parsing = true;
                    state.parsed_successfully = false;
                    state.lines_read = 0;
                    state.error_count = 0;
                    state.parse_failed_message = None;
                    state.errors.clear();
                    state.warnings.clear();
                    state.warnings_acknowledged = false;
                    state.parsed_contest_state = None;
                }
                Ok(ParserEvent::Progress { lines_read }) => {
                    state.lines_read = lines_read;
                }
                Ok(ParserEvent::LineError { line_no, message }) => {
                    state.error_count += 1;
                    let msg = format!("Line {line_no}: {message}");
                    state.errors.push(msg);
                    if state.errors.len() > 8 {
                        state.errors.remove(0);
                    }
                }
                Ok(ParserEvent::Finished {
                    lines_read,
                    error_count,
                    contest_state,
                    warnings,
                }) => {
                    state.is_parsing = false;
                    state.lines_read = lines_read;
                    state.error_count = error_count;
                    state.parsed_successfully = error_count == 0;
                    if error_count > 0 {
                        state.parse_failed_message =
                            Some(format!("Parsing finished with {error_count} JSON error(s)"));
                        state.parsed_contest_state = None;
                        state.parsed_config = None;
                        state.warnings.clear();
                        state.warnings_acknowledged = false;
                    } else {
                        state.parse_failed_message = None;
                        state.parsed_contest_state = Some(*contest_state);
                        state.warnings = warnings;
                        state.warnings_acknowledged = false;
                    }
                    state.receiver = None;
                    break;
                }
                Ok(ParserEvent::Failed { message }) => {
                    state.is_parsing = false;
                    state.parsed_successfully = false;
                    state.parse_failed_message = Some(message.clone());
                    state.errors.push(message);
                    state.warnings.clear();
                    state.warnings_acknowledged = false;
                    state.parsed_contest_state = None;
                    state.parsed_config = None;
                    if state.errors.len() > 8 {
                        state.errors.remove(0);
                    }
                    state.receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    state.is_parsing = false;
                    state.parsed_successfully = false;
                    state.parse_failed_message = Some("Parser thread disconnected".to_string());
                    state.receiver = None;
                    state.warnings.clear();
                    state.warnings_acknowledged = false;
                    state.parsed_contest_state = None;
                    state.parsed_config = None;
                    break;
                }
            }
        }

        ui.ctx().request_repaint();
    }

    ui.add_space(8.0);
    let can_parse = current_path.is_some() && !state.is_parsing;
    if ui
        .add_enabled(can_parse, egui::Button::new("Parse"))
        .clicked()
        && let Some(folder_path) = current_path.clone()
    {
        match validate_cdp_folder(&folder_path) {
            Ok(event_feed_path) => match config_loader::load_pyrite_config(&folder_path) {
                Ok(config) => {
                    let parser_config = config.clone();
                    state.is_parsing = true;
                    state.parsed_successfully = false;
                    state.parsed_path = Some(folder_path);
                    state.lines_read = 0;
                    state.error_count = 0;
                    state.parse_failed_message = None;
                    state.errors.clear();
                    state.warnings.clear();
                    state.warnings_acknowledged = false;
                    state.parsed_contest_state = None;
                    state.parsed_config = Some(config);
                    state.receiver = Some(spawn_event_feed_parser(event_feed_path, parser_config));
                    ui.ctx().request_repaint();
                }
                Err(message) => {
                    state.is_parsing = false;
                    state.parsed_successfully = false;
                    state.parsed_path = Some(folder_path);
                    state.lines_read = 0;
                    state.error_count = 0;
                    state.parse_failed_message = Some("Invalid config.toml".to_string());
                    state.errors = vec![message];
                    state.warnings.clear();
                    state.warnings_acknowledged = false;
                    state.parsed_contest_state = None;
                    state.parsed_config = None;
                    state.receiver = None;
                }
            },
            Err(validation_errors) => {
                state.is_parsing = false;
                state.parsed_successfully = false;
                state.parsed_path = Some(folder_path);
                state.lines_read = 0;
                state.error_count = 0;
                state.parse_failed_message = Some("Invalid CDP folder structure".to_string());
                state.errors = validation_errors;
                state.warnings.clear();
                state.warnings_acknowledged = false;
                state.parsed_contest_state = None;
                state.parsed_config = None;
                state.receiver = None;
            }
        }
    }

    ui.add_space(8.0);
    if state.is_parsing {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new());
            ui.label(format!(
                "Parsing... lines: {} | errors: {}",
                state.lines_read, state.error_count
            ));
        });
    } else if state.parsed_successfully {
        ui.colored_label(
            egui::Color32::LIGHT_GREEN,
            format!("Parse completed. lines: {} | errors: 0", state.lines_read),
        );
    } else if let Some(msg) = &state.parse_failed_message {
        ui.colored_label(egui::Color32::LIGHT_RED, msg);
    }

    if !state.errors.is_empty() {
        ui.add_space(8.0);
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(58, 22, 22))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 60, 60)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Parse Errors").strong());
                for err in &state.errors {
                    ui.colored_label(egui::Color32::from_rgb(255, 170, 170), err);
                }
            });
    }

    if !state.warnings.is_empty() {
        ui.add_space(8.0);
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(56, 48, 20))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 160, 70)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Parse Warnings").strong());
                for warning in &state.warnings {
                    ui.colored_label(egui::Color32::from_rgb(255, 220, 140), warning);
                }
            });

        ui.add_space(8.0);
        if !state.warnings_acknowledged
            && ui.button("Proceed despite warnings").clicked()
        {
            state.warnings_acknowledged = true;
        }
    }

    ui.add_space(8.0);
    let can_continue = state.parsed_successfully
        && !state.is_parsing
        && current_path.is_some()
        && current_path == state.parsed_path
        && (state.warnings.is_empty() || state.warnings_acknowledged);
    if ui
        .add_enabled(can_continue, egui::Button::new("Continue"))
        .clicked()
    {
        return LoadDataAction::Continue;
    }

    LoadDataAction::Stay
}
