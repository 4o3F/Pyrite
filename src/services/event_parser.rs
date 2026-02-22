use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::mpsc::{self, Receiver, Sender};

use serde::de::DeserializeOwned;
use tracing::{info, warn};

use crate::models;
use crate::services::config_loader::PyriteConfig;
use crate::services::contest_processor;

#[derive(Debug)]
pub enum ParserEvent {
    Started,
    Progress {
        lines_read: u64,
    },
    LineError {
        line_no: u64,
        message: String,
    },
    Finished {
        lines_read: u64,
        error_count: u64,
        contest_state: Box<models::ContestState>,
        warnings: Vec<String>,
    },
    Failed {
        message: String,
    },
}

fn handle_event<T>(
    name: &str,
    line_no: u64,
    event_data: serde_json::Value,
    state_map: &mut HashMap<String, T>,
    contest_defined: bool,
) -> Result<(), String>
where
    T: Clone + DeserializeOwned + models::HasId,
{
    if !contest_defined {
        return Err("Wrong event feed: contest not defined yet".to_string());
    }

    let data: T = serde_json::from_value(event_data.clone()).map_err(|err| {
        format!(
            "Line {}: failed to parse {} payload: {} | data: {:#?}",
            line_no, name, err, event_data
        )
    })?;

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
}

fn emit_line_error(tx: &Sender<ParserEvent>, line_no: u64, message: impl Into<String>) -> u64 {
    let _ = tx.send(ParserEvent::LineError {
        line_no,
        message: message.into(),
    });
    1
}

fn apply_event_result(tx: &Sender<ParserEvent>, line_no: u64, result: Result<(), String>) -> u64 {
    if let Err(err) = result {
        return emit_line_error(tx, line_no, err);
    }
    0
}

fn parse_event_line(
    tx: &Sender<ParserEvent>,
    line_no: u64,
    line: &str,
    state: &mut models::ContestState,
) -> u64 {
    let event = match serde_json::from_str::<models::Event>(line) {
        Ok(event) => event,
        Err(err) => return emit_line_error(tx, line_no, err.to_string()),
    };

    let Some(event_data) = event.data else {
        warn!(
            "Empty data for event {:?} on line {}",
            event.event_type, line_no
        );
        return 0;
    };

    match event.event_type {
        models::EventType::Contest => match serde_json::from_value::<models::Contest>(event_data) {
            Ok(mut data) => {
                data.scoreboard_freeze_time = data
                    .start_time
                    .map(|start| start + (data.duration - data.scoreboard_freeze_duration));
                if state.contest.is_some() {
                    info!("Updating contest data");
                } else {
                    info!("New contest data parsed");
                }
                state.contest = Some(data);
                0
            }
            Err(err) => {
                emit_line_error(tx, line_no, format!("Failed to parse contest data: {err}"))
            }
        },
        models::EventType::JudgementTypes => apply_event_result(
            tx,
            line_no,
            handle_event(
                "judgement types",
                line_no,
                event_data,
                &mut state.judgement_types,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Languages => {
            info!("Skipping useless languages defination on line {}", line_no);
            0
        }
        models::EventType::Groups => apply_event_result(
            tx,
            line_no,
            handle_event(
                "groups",
                line_no,
                event_data,
                &mut state.groups,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Organizations => apply_event_result(
            tx,
            line_no,
            handle_event(
                "organizations",
                line_no,
                event_data,
                &mut state.organizations,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Teams => apply_event_result(
            tx,
            line_no,
            handle_event(
                "teams",
                line_no,
                event_data,
                &mut state.teams,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Accounts => apply_event_result(
            tx,
            line_no,
            handle_event(
                "accounts",
                line_no,
                event_data,
                &mut state.accounts,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Problems => apply_event_result(
            tx,
            line_no,
            handle_event(
                "problems",
                line_no,
                event_data,
                &mut state.problems,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Runs => {
            info!("Skipping useless run detail on line {}", line_no);
            0
        }
        models::EventType::Submissions => apply_event_result(
            tx,
            line_no,
            handle_event(
                "submissions",
                line_no,
                event_data,
                &mut state.submissions,
                state.contest.is_some(),
            ),
        ),
        models::EventType::Judgements => apply_event_result(
            tx,
            line_no,
            handle_event(
                "judgements",
                line_no,
                event_data,
                &mut state.judgements,
                state.contest.is_some(),
            ),
        ),
        models::EventType::State => {
            warn!("Skipping state change notify on line {}", line_no);
            0
        }
        models::EventType::Clarifications => {
            warn!("Skipping clarification on line {}", line_no);
            0
        }
        models::EventType::Awards => apply_event_result(
            tx,
            line_no,
            handle_event(
                "awards",
                line_no,
                event_data,
                &mut state.awards,
                state.contest.is_some(),
            ),
        ),
        event_type => emit_line_error(
            tx,
            line_no,
            format!("Unexpected event type {:?} on line {}", event_type, line_no),
        ),
    }
}

pub fn spawn_event_feed_parser(path: String, config: PyriteConfig) -> Receiver<ParserEvent> {
    let (tx, rx) = mpsc::channel::<ParserEvent>();

    std::thread::spawn(move || {
        let _ = tx.send(ParserEvent::Started);

        let file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                let _ = tx.send(ParserEvent::Failed {
                    message: format!("Failed to open file '{path}': {err}"),
                });
                return;
            }
        };

        let reader = BufReader::new(file);
        let mut lines_read: u64 = 0;
        let mut error_count: u64 = 0;
        let mut state = models::ContestState::new();

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    lines_read += 1;
                    error_count += parse_event_line(&tx, lines_read, &line, &mut state);

                    if lines_read.is_multiple_of(100) {
                        let _ = tx.send(ParserEvent::Progress { lines_read });
                    }
                }
                Err(err) => {
                    let _ = tx.send(ParserEvent::Failed {
                        message: format!("Failed while reading file '{path}': {err}"),
                    });
                    return;
                }
            }
        }

        let warnings = match contest_processor::validate_and_transform(&mut state, &config) {
            Ok(warnings) => warnings,
            Err(message) => {
                let _ = tx.send(ParserEvent::Failed { message });
                return;
            }
        };

        let _ = tx.send(ParserEvent::Finished {
            lines_read,
            error_count,
            contest_state: Box::new(state),
            warnings,
        });
    });

    rx
}
