use eframe::egui;
use rfd::FileDialog;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::{Mutex, OnceLock};

use crate::models::{Award, ContestState};

pub enum SetAwardAction {
    Stay,
    Continue,
}

struct SetAwardUiState {
    selected_group_ids: BTreeMap<String, bool>,
    last_group_key: String,
    medal_gold_count: usize,
    medal_silver_count: usize,
    medal_bronze_count: usize,
    medal_gold_citation: String,
    medal_silver_citation: String,
    medal_bronze_citation: String,
    award_id: String,
    citation: String,
    team_ids_csv: String,
    message: Option<String>,
}

impl Default for SetAwardUiState {
    fn default() -> Self {
        Self {
            selected_group_ids: BTreeMap::new(),
            last_group_key: String::new(),
            medal_gold_count: 0,
            medal_silver_count: 0,
            medal_bronze_count: 0,
            medal_gold_citation: "Gold Medal".to_string(),
            medal_silver_citation: "Silver Medal".to_string(),
            medal_bronze_citation: "Bronze Medal".to_string(),
            award_id: String::new(),
            citation: String::new(),
            team_ids_csv: String::new(),
            message: None,
        }
    }
}

static SET_AWARD_UI_STATE: OnceLock<Mutex<SetAwardUiState>> = OnceLock::new();

fn set_award_ui_state() -> &'static Mutex<SetAwardUiState> {
    SET_AWARD_UI_STATE.get_or_init(|| Mutex::new(SetAwardUiState::default()))
}

fn compute_group_key(contest_state: &ContestState) -> String {
    let mut items: Vec<(i32, String, String)> = contest_state
        .groups
        .values()
        .map(|group| (group.sortorder, group.name.clone(), group.id.clone()))
        .collect();
    items.sort_by(|a, b| a.cmp(b));
    items
        .into_iter()
        .map(|(sortorder, name, id)| format!("{sortorder}:{name}:{id}"))
        .collect::<Vec<_>>()
        .join("|")
}

fn sorted_group_ids(contest_state: &ContestState) -> Vec<String> {
    let mut group_items: Vec<_> = contest_state.groups.values().collect();
    group_items.sort_by(|a, b| {
        a.sortorder
            .cmp(&b.sortorder)
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });
    group_items
        .into_iter()
        .map(|group| group.id.clone())
        .collect()
}

fn sync_group_selection(state: &mut SetAwardUiState, contest_state: &ContestState) {
    let current_key = compute_group_key(contest_state);
    let group_ids = sorted_group_ids(contest_state);
    let known_ids: HashSet<String> = group_ids.iter().cloned().collect();

    state
        .selected_group_ids
        .retain(|group_id, _| known_ids.contains(group_id));

    if state.last_group_key != current_key {
        state.selected_group_ids.clear();
        for group_id in group_ids {
            state.selected_group_ids.insert(group_id, true);
        }
        state.last_group_key = current_key;
        return;
    }

    for group_id in group_ids {
        state.selected_group_ids.entry(group_id).or_insert(true);
    }
}

fn build_medal_preview(
    contest_state: &ContestState,
    selected_group_ids: &BTreeMap<String, bool>,
    gold_count: usize,
    silver_count: usize,
    bronze_count: usize,
) -> (Vec<(String, String)>, Vec<(String, String)>, Vec<(String, String)>, usize) {
    let selected_groups: HashSet<&str> = selected_group_ids
        .iter()
        .filter_map(|(group_id, selected)| if *selected { Some(group_id.as_str()) } else { None })
        .collect();

    let eligible: Vec<(String, String)> = contest_state
        .leaderboard_finalized
        .iter()
        .filter_map(|team_status| {
            let team = contest_state.teams.get(&team_status.team_id)?;
            let is_eligible = team
                .group_ids
                .iter()
                .any(|group_id| selected_groups.contains(group_id.as_str()));
            if is_eligible {
                Some((team_status.team_id.clone(), team_status.team_name.clone()))
            } else {
                None
            }
        })
        .collect();

    let gold_end = gold_count.min(eligible.len());
    let silver_end = (gold_end + silver_count).min(eligible.len());
    let bronze_end = (silver_end + bronze_count).min(eligible.len());

    let gold = eligible[0..gold_end].to_vec();
    let silver = eligible[gold_end..silver_end].to_vec();
    let bronze = eligible[silver_end..bronze_end].to_vec();

    (gold, silver, bronze, eligible.len())
}

fn show_medal_scroll(ui: &mut egui::Ui, id_salt: &str, title: &str, teams: &[(String, String)]) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(title);
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt(id_salt)
                .max_height(120.0)
                .show(ui, |ui| {
                    if teams.is_empty() {
                        ui.label("No teams.");
                    } else {
                        for (team_id, team_name) in teams {
                            ui.label(format!("{team_id} | {team_name}"));
                        }
                    }
                });
        });
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).strong());
}

fn save_awards_to_file(contest_state: &ContestState) -> Result<String, String> {
    let Some(path) = FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name("awards.json")
        .save_file()
    else {
        return Ok("Save canceled".to_string());
    };

    let json = serde_json::to_string_pretty(&contest_state.awards)
        .map_err(|err| format!("Failed to serialize awards: {err}"))?;
    fs::write(&path, json)
        .map_err(|err| format!("Failed to write awards file {}: {err}", path.display()))?;

    Ok(format!("Saved awards to {}", path.display()))
}

fn load_awards_from_file(contest_state: &mut ContestState) -> Result<String, String> {
    let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() else {
        return Ok("Load canceled".to_string());
    };

    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read awards file {}: {err}", path.display()))?;

    let parsed: std::collections::HashMap<String, Award> = serde_json::from_str(&raw)
        .map_err(|err| format!("Failed to parse awards JSON: {err}"))?;

    let mut normalized = std::collections::HashMap::with_capacity(parsed.len());
    for (_key, award) in parsed {
        normalized.insert(award.id.clone(), award);
    }

    contest_state.awards = normalized;
    Ok(format!(
        "Loaded {} award(s) from {}",
        contest_state.awards.len(),
        path.display()
    ))
}

pub fn ui(ui: &mut egui::Ui, contest_state: &mut ContestState) -> SetAwardAction {
    let mut action = SetAwardAction::Stay;
    egui::ScrollArea::vertical()
        .id_salt("set_award_screen_scroll")
        .show(ui, |ui| {
            ui.heading("Set Award");
            ui.add_space(8.0);
            ui.label("Configure award settings for the presentation.");
            ui.label(format!(
                "Current awards in state: {}",
                contest_state.awards.len()
            ));
            ui.separator();
            ui.add_space(8.0);

            let mut state = set_award_ui_state()
                .lock()
                .expect("set award ui state lock poisoned");

            ui.horizontal(|ui| {
                if ui.button("Save Awards").clicked() {
                    state.message = Some(match save_awards_to_file(contest_state) {
                        Ok(msg) => msg,
                        Err(err) => err,
                    });
                }
                if ui.button("Load Awards").clicked() {
                    state.message = Some(match load_awards_from_file(contest_state) {
                        Ok(msg) => msg,
                        Err(err) => err,
                    });
                }
            });
            ui.add_space(10.0);

            sync_group_selection(&mut state, contest_state);

            let (gold_preview, silver_preview, bronze_preview, eligible_count) =
                build_medal_preview(
                    contest_state,
                    &state.selected_group_ids,
                    state.medal_gold_count,
                    state.medal_silver_count,
                    state.medal_bronze_count,
                );

            let requested_total =
                state.medal_gold_count + state.medal_silver_count + state.medal_bronze_count;

            ui.columns(3, |columns| {
                egui::Frame::group(columns[0].style())
                    .inner_margin(egui::Margin::same(10))
                    .show(&mut columns[0], |ui| {
                        section_title(ui, "Categories for medal calculation");
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button("Select All").clicked() {
                                for selected in state.selected_group_ids.values_mut() {
                                    *selected = true;
                                }
                            }
                            if ui.button("Clear All").clicked() {
                                for selected in state.selected_group_ids.values_mut() {
                                    *selected = false;
                                }
                            }
                        });
                        ui.separator();
                        ui.add_space(6.0);

                        if contest_state.groups.is_empty() {
                            ui.label("No groups available.");
                        } else {
                            let sorted_group_ids = sorted_group_ids(contest_state);
                            egui::ScrollArea::vertical()
                                .id_salt("category_group_scroll")
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    for group_id in sorted_group_ids {
                                        if let Some(group) = contest_state.groups.get(&group_id)
                                            && let Some(selected) =
                                                state.selected_group_ids.get_mut(&group_id)
                                        {
                                            ui.checkbox(
                                                selected,
                                                format!("{} ({})", group.name, group.id),
                                            );
                                        }
                                    }
                                });
                        }
                    });

                egui::Frame::group(columns[1].style())
                    .inner_margin(egui::Margin::same(10))
                    .show(&mut columns[1], |ui| {
                        section_title(ui, "Medal setup and preview");
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Gold count");
                            ui.add(egui::DragValue::new(&mut state.medal_gold_count).range(0..=usize::MAX));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Silver count");
                            ui.add(
                                egui::DragValue::new(&mut state.medal_silver_count).range(0..=usize::MAX),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Bronze count");
                            ui.add(
                                egui::DragValue::new(&mut state.medal_bronze_count).range(0..=usize::MAX),
                            );
                        });

                        ui.separator();
                        ui.add_space(4.0);
                        ui.label("Gold citation");
                        ui.add(egui::TextEdit::singleline(&mut state.medal_gold_citation));
                        ui.label("Silver citation");
                        ui.add(egui::TextEdit::singleline(&mut state.medal_silver_citation));
                        ui.label("Bronze citation");
                        ui.add(egui::TextEdit::singleline(&mut state.medal_bronze_citation));

                        ui.separator();
                        ui.add_space(4.0);
                        ui.label(format!("Eligible teams: {eligible_count}"));
                        if requested_total > eligible_count {
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!(
                                    "Requested medals ({requested_total}) exceed eligible teams ({eligible_count})."
                                ),
                            );
                        }
                        ui.add_space(6.0);
                        show_medal_scroll(ui, "gold_winner_scroll", "Gold winners", &gold_preview);
                        ui.add_space(4.0);
                        show_medal_scroll(ui, "silver_winner_scroll", "Silver winners", &silver_preview);
                        ui.add_space(4.0);
                        show_medal_scroll(ui, "bronze_winner_scroll", "Bronze winners", &bronze_preview);

                        ui.add_space(8.0);
                        if ui.button("Apply Medal Awards").clicked() {
                            let gold_team_ids: Vec<String> =
                                gold_preview.iter().map(|(id, _)| id.clone()).collect();
                            let silver_team_ids: Vec<String> =
                                silver_preview.iter().map(|(id, _)| id.clone()).collect();
                            let bronze_team_ids: Vec<String> =
                                bronze_preview.iter().map(|(id, _)| id.clone()).collect();

                            contest_state.awards.insert(
                                "medal-gold".to_string(),
                                Award {
                                    id: "medal-gold".to_string(),
                                    citation: state.medal_gold_citation.trim().to_string(),
                                    team_ids: gold_team_ids,
                                },
                            );
                            contest_state.awards.insert(
                                "medal-silver".to_string(),
                                Award {
                                    id: "medal-silver".to_string(),
                                    citation: state.medal_silver_citation.trim().to_string(),
                                    team_ids: silver_team_ids,
                                },
                            );
                            contest_state.awards.insert(
                                "medal-bronze".to_string(),
                                Award {
                                    id: "medal-bronze".to_string(),
                                    citation: state.medal_bronze_citation.trim().to_string(),
                                    team_ids: bronze_team_ids,
                                },
                            );
                            state.message = Some("Medal awards applied to contest state".to_string());
                        }
                    });

                egui::Frame::group(columns[2].style())
                    .inner_margin(egui::Margin::same(10))
                    .show(&mut columns[2], |ui| {
                        section_title(ui, "Current awards");
                        ui.separator();
                        ui.add_space(6.0);

                        let mut sorted_awards: Vec<_> = contest_state.awards.values().cloned().collect();
                        sorted_awards.sort_by(|a, b| a.id.cmp(&b.id));

                        let mut delete_award_id: Option<String> = None;
                        egui::ScrollArea::vertical()
                            .id_salt("current_awards_scroll")
                            .max_height(430.0)
                            .show(ui, |ui| {
                                if sorted_awards.is_empty() {
                                    ui.label("No awards configured.");
                                    return;
                                }

                                for award in &sorted_awards {
                                    ui.push_id(&award.id, |ui| {
                                        egui::Frame::group(ui.style())
                                            .inner_margin(egui::Margin::same(8))
                                            .show(ui, |ui| {
                                                ui.label(format!("ID: {}", award.id));
                                                ui.label(format!("Citation: {}", award.citation));
                                                ui.label(format!("Teams: {}", award.team_ids.len()));
                                                let preview = if award.team_ids.is_empty() {
                                                    "None".to_string()
                                                } else {
                                                    let ids: Vec<&str> = award
                                                        .team_ids
                                                        .iter()
                                                        .take(5)
                                                        .map(String::as_str)
                                                        .collect();
                                                    let mut compact = ids.join(", ");
                                                    if award.team_ids.len() > 5 {
                                                        compact.push_str(" ...");
                                                    }
                                                    compact
                                                };
                                                ui.label(format!("Team IDs: {preview}"));
                                                ui.add_space(4.0);
                                                if ui.button("Delete").clicked() {
                                                    delete_award_id = Some(award.id.clone());
                                                }
                                            });
                                    });
                                    ui.add_space(8.0);
                                }
                            });

                        if let Some(award_id) = delete_award_id {
                            contest_state.awards.remove(&award_id);
                            state.message = Some(format!("Deleted award {award_id}"));
                        }
                    });
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    section_title(ui, "Manual custom award");
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label("Award ID");
                    let manual_width = ui.available_width().max(300.0);
                    ui.add_sized(
                        [manual_width, 28.0],
                        egui::TextEdit::singleline(&mut state.award_id),
                    );
                    ui.add_space(8.0);

                    ui.label("Citation");
                    ui.add_sized(
                        [manual_width, 28.0],
                        egui::TextEdit::singleline(&mut state.citation),
                    );
                    ui.add_space(8.0);

                    ui.label("Team IDs (comma separated)");
                    ui.add_sized(
                        [manual_width, 28.0],
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
                            state.message = Some(
                                "Award ID, citation, and at least one team ID are required".to_string(),
                            );
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
                });

            if let Some(message) = &state.message {
                ui.add_space(10.0);
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(message);
                    });
                }
            ui.add_space(12.0);

            if ui.button("Present").clicked() {
                action = SetAwardAction::Continue;
            }
        });

    action
}
