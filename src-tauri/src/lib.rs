mod models;

use models::{Contest, Event, EventType, HasId, PyriteState, TeamStatus};
use std::{
    collections::{BinaryHeap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::PathBuf,
};
use tauri::ipc::Channel;

use anyhow::Result;
use tracing::{error, info, warn};
use tracing_unwrap::{OptionExt, ResultExt};

use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

use crate::models::{Judgement, Problem};

fn handle_event<T>(
    name: &str,
    event_data: serde_json::Value,
    state_map: &mut HashMap<String, T>,
    contest_defined: bool,
) -> Result<(), String>
where
    T: Clone + DeserializeOwned + HasId,
{
    let data: T = serde_json::from_value(event_data.clone()).expect_or_log(&format!(
        "Failed to parse {} original {:#?}",
        name, event_data
    ));

    if contest_defined {
        match state_map.entry(data.id().to_string()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                warn!("Updating existing {} {}", name, data.id());
                entry.insert(data.clone());
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(data.clone());
                info!("Added new {} {}", name, data.id());
            }
        }
        Ok(())
    } else {
        error!("Wrong event feed, contest not defined yet!");
        Err("Wrong event feed, contest not defined yet!".to_string())
    }
}

#[tauri::command]
async fn parse_event_feed(
    input_path: String,
    output_path: String,
    log_channel: Channel<f64>,
) -> Result<(), String> {
    info!("Input path: {}", input_path);
    info!("Output path: {}", output_path);

    let output_path = PathBuf::from(output_path);
    if output_path.exists() && output_path.is_dir() {
        error!("Output path exist and is a dir!");
        return Err("Output path exist and is a dir!".to_string());
    }

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect_or_log("Failed to create output dir");
        }
    }

    let mut output_path = File::create(output_path).expect("Failed to create output file");

    let mut file: File = File::open(input_path).expect_or_log("Failed to open event feed file");
    let reader = BufReader::new(&file);
    let total_lines = reader.lines().count();
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut state = PyriteState::new();
    for (line_num, line) in reader.lines().enumerate() {
        let line = line.expect_or_log("Failed to read line from event feed file");
        let line_num = line_num + 1;

        if line_num % 50 == 0 {
            log_channel
                .send(f64::from(line_num as i32) / f64::from(total_lines as i32))
                .map_err(|e| e.to_string())?;
        }

        if line.trim().is_empty() {
            warn!("Empty line encountered on line {}", line_num);
            continue;
        }
        match serde_json::from_str::<Event>(&line) {
            Ok(event) => match event.data {
                Some(event_data) => match event.event_type {
                    EventType::Contest => {
                        let mut data = serde_json::from_value::<Contest>(event_data)
                            .expect_or_log("Failed to parse contest data");
                        data.scoreboard_freeze_time = data
                            .start_time
                            .map(|start| start + (data.duration - data.scoreboard_freeze_duration));
                        if state.contest.is_some() {
                            info!("Updating contest data")
                        } else {
                            info!("New contest data parsed");
                        }
                        state.contest = Some(data);
                    }
                    EventType::JudgementTypes => handle_event(
                        "judgement types",
                        event_data,
                        &mut state.judgement_types,
                        state.contest.is_some(),
                    )?,
                    EventType::Languages => {
                        info!("Skipping useless languages defination on line {}", line_num);
                    }
                    EventType::Groups => handle_event(
                        "groups",
                        event_data,
                        &mut state.groups,
                        state.contest.is_some(),
                    )?,
                    EventType::Organizations => handle_event(
                        "organizations",
                        event_data,
                        &mut state.organizations,
                        state.contest.is_some(),
                    )?,
                    EventType::Teams => handle_event(
                        "teams",
                        event_data,
                        &mut state.teams,
                        state.contest.is_some(),
                    )?,
                    EventType::Accounts => handle_event(
                        "accounts",
                        event_data,
                        &mut state.accounts,
                        state.contest.is_some(),
                    )?,
                    EventType::Problems => handle_event(
                        "problems",
                        event_data,
                        &mut state.problems,
                        state.contest.is_some(),
                    )?,
                    EventType::Runs => {
                        info!("Skipping useless run detail on line {}", line_num);
                    }
                    EventType::Submissions => handle_event(
                        "submissions",
                        event_data,
                        &mut state.submissions,
                        state.contest.is_some(),
                    )?,
                    EventType::Judgements => handle_event(
                        "judgements",
                        event_data,
                        &mut state.judgements,
                        state.contest.is_some(),
                    )?,
                    EventType::State => {
                        warn!("Skipping state change notify on line {}", line_num);
                    }
                    EventType::Clarifications => {
                        warn!("Skipping clarification on line {}", line_num);
                    }
                    EventType::Awards => handle_event(
                        "awards",
                        event_data,
                        &mut state.awards,
                        state.contest.is_some(),
                    )?,

                    event_type => {
                        error!(
                            "Unexpected event type {:?} on line {}, maybe wrong contest API version?",
                            event_type, line_num
                        );
                        // info!("Current state data {:#?}", state);
                        break;
                    }
                },
                None => {
                    warn!(
                        "Empty data for event {:?} on line {}",
                        event.event_type, line_num
                    )
                }
            },
            Err(err) => {
                error!("Failed to parse line {}, error: {}", line_num, err);
            }
        }
    }

    info!("Event feed parse complete, validating...");
    // Do validation, check if all submisssions has been judged
    if state.submissions.len() != state.judgements.len() {
        let judged_submission_ids = state
            .judgements
            .values()
            .map(|j| j.submission_id.clone())
            .collect::<HashSet<String>>();
        for submission_id in state.submissions.keys() {
            if !judged_submission_ids.contains(submission_id) {
                error!("Submission {} not judged!", submission_id);
                return Err(format!("Submission {} not judged", submission_id));
            }
        }
    }

    // Calculate scoreboard before frozen
    let mut team_status_map: HashMap<String, TeamStatus> = HashMap::new();
    for team in state.teams.values() {
        let sortorder = team
            .group_ids
            .iter()
            .filter_map(|group_id| state.groups.get(group_id))
            .map(|group| group.sortorder)
            .min()
            .unwrap_or(0);

        let status = TeamStatus::new(
            team.id.clone(),
            team.name.clone(),
            team.organization_id
                .clone()
                .expect_or_log(format!("Missing organization_id for team {}", team.id).as_str()),
            sortorder,
        );
        team_status_map.insert(team.id.clone(), status);
    }

    let contest_start_time = if let Some(contest) = &state.contest {
        contest.start_time
    } else {
        error!("Contest start time not defined");
        return Err("Contest start time not defined".to_string());
    };

    let contest_freeze_time = if let Some(contest) = &state.contest {
        contest.scoreboard_freeze_time
    } else {
        error!("Contest freeze time not defined");
        return Err("Contest freeze time not defined".to_string());
    };

    // Handle all judgements before board freeze
    let mut judgements = state.judgements.values().collect::<Vec<&Judgement>>();
    judgements.sort_by(|&j1, &j2| {
        let s1 = state.submissions.get(&j1.submission_id);
        let s2 = state.submissions.get(&j2.submission_id);
        // Fallback to use judge time if the event feed is corrupted
        let s1_time = if let Some(s1) = s1 {
            s1.time
        } else {
            j1.start_time
        };
        let s2_time = if let Some(s2) = s2 {
            s2.time
        } else {
            j2.start_time
        };
        s1_time.cmp(&s2_time)
    });

    for judgement in judgements {
        if let Some(submission) = state.submissions.get(&judgement.submission_id) {
            if let Some(team_status) = team_status_map.get_mut(&submission.team_id) {
                if let Some(submission) = state.submissions.get(&judgement.submission_id) {
                    let submission_time = submission.time.expect_or_log("Unknown submission time");
                    team_status.add_submission(
                        &submission.problem_id,
                        submission_time,
                        judgement.judgement_type_id.as_deref(),
                        &state.judgement_types,
                        contest_start_time,
                        contest_freeze_time,
                    );
                } else {
                    error!("Unknown submission id {}", judgement.submission_id);
                    unreachable!()
                }
            }
        }
    }

    let mut leaderboard: BinaryHeap<TeamStatus> = team_status_map.into_values().collect();

    let mut sorted_leaderboard = Vec::new();
    while let Some(team) = leaderboard.pop() {
        sorted_leaderboard.push(team);
    }
    sorted_leaderboard.reverse();

    for (rank, item) in sorted_leaderboard.iter().enumerate() {
        let rank = rank + 1;
        info!(
            "Rank {:0>3} Penalty {} TeamName: {}",
            rank, item.total_penalty, item.team_name
        );
    }

    #[derive(Debug, Clone, Serialize)]
    struct Result {
        scoreboard: Vec<TeamStatus>,
        problems: Vec<Problem>,
        awards: HashMap<String, Vec<String>>,
    }

    let awards =
        state
            .awards
            .iter()
            .fold(HashMap::<String, Vec<String>>::new(), |mut map, award| {
                for team_id in award.1.team_ids.clone() {
                    let entry = map.entry(team_id).or_default();
                    entry.push(award.1.citation.clone());
                }
                map
            });

    let result = Result {
        scoreboard: sorted_leaderboard,
        problems: state.problems.values().map(|x| x.clone()).collect(),
        awards: awards,
    };

    let serialized_leaderboard =
        serde_json::to_string_pretty(&result).expect_or_log("Failed to serialize final data");

    output_path
        .write_all(serialized_leaderboard.as_bytes())
        .expect_or_log("Failed to write serialized data");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![parse_event_feed])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
