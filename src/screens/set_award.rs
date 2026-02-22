use eframe::egui;
use std::sync::{Mutex, OnceLock};

use crate::models::{Award, ContestState};

pub enum SetAwardAction {
    Stay,
    Continue,
}

#[derive(Default)]
struct SetAwardUiState {
    award_id: String,
    citation: String,
    team_ids_csv: String,
    message: Option<String>,
}

static SET_AWARD_UI_STATE: OnceLock<Mutex<SetAwardUiState>> = OnceLock::new();

fn set_award_ui_state() -> &'static Mutex<SetAwardUiState> {
    SET_AWARD_UI_STATE.get_or_init(|| Mutex::new(SetAwardUiState::default()))
}

pub fn ui(ui: &mut egui::Ui, contest_state: &mut ContestState) -> SetAwardAction {
    ui.heading("Set Award");
    ui.add_space(8.0);
    ui.label("Configure award settings for the presentation.");
    ui.label(format!(
        "Current awards in state: {}",
        contest_state.awards.len()
    ));
    ui.add_space(12.0);

    let mut state = set_award_ui_state()
        .lock()
        .expect("set award ui state lock poisoned");

    ui.label("Award ID");
    ui.add_sized(
        [500.0, 28.0],
        egui::TextEdit::singleline(&mut state.award_id),
    );
    ui.add_space(6.0);

    ui.label("Citation");
    ui.add_sized(
        [500.0, 28.0],
        egui::TextEdit::singleline(&mut state.citation),
    );
    ui.add_space(6.0);

    ui.label("Team IDs (comma separated)");
    ui.add_sized(
        [500.0, 28.0],
        egui::TextEdit::singleline(&mut state.team_ids_csv),
    );
    ui.add_space(10.0);

    if ui.button("Add/Update Award").clicked() {
        let award_id = state.award_id.trim().to_string();
        let citation = state.citation.trim().to_string();
        let team_ids: Vec<String> = state
            .team_ids_csv
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        if award_id.is_empty() || citation.is_empty() || team_ids.is_empty() {
            state.message =
                Some("Award ID, citation, and at least one team ID are required".to_string());
        } else {
            contest_state.awards.insert(
                award_id.clone(),
                Award {
                    id: award_id,
                    citation,
                    team_ids,
                },
            );
            state.message = Some("Award upserted to contest state".to_string());
        }
    }

    if let Some(message) = &state.message {
        ui.add_space(8.0);
        ui.label(message);
    }

    ui.add_space(12.0);

    if ui.button("Present").clicked() {
        return SetAwardAction::Continue;
    }

    SetAwardAction::Stay
}
