use std::collections::{BinaryHeap, HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use tracing::{error, info, warn};

use crate::services::config_loader::PyriteConfig;
use crate::models::{ContestState, Judgement, TeamStatus};

fn apply_submission_filters(state: &mut ContestState, config: &PyriteConfig) {
    if config.filter_team_submissions.is_empty() {
        return;
    }

    let filter_set: HashSet<&str> = config
        .filter_team_submissions
        .iter()
        .map(String::as_str)
        .collect();

    let removed_submission_ids: HashSet<String> = state
        .submissions
        .iter()
        .filter(|(_, submission)| filter_set.contains(submission.team_id.as_str()))
        .map(|(submission_id, _)| submission_id.clone())
        .collect();

    if removed_submission_ids.is_empty() {
        info!("No submissions matched filter_team_submissions");
        return;
    }

    info!("Removing submissions {:?}", removed_submission_ids);

    state
        .submissions
        .retain(|submission_id, _| !removed_submission_ids.contains(submission_id));
    state
        .judgements
        .retain(|_, judgement| !removed_submission_ids.contains(&judgement.submission_id));

    info!(
        "Filtered out {} submissions and related judgements for teams {:?}",
        removed_submission_ids.len(),
        config.filter_team_submissions
    );
}

fn validate_all_submissions_judged(state: &ContestState) -> Result<(), String> {
    let judged_submission_ids = state
        .judgements
        .values()
        .map(|j| j.submission_id.clone())
        .collect::<HashSet<String>>();

    for submission_id in state.submissions.keys() {
        if !judged_submission_ids.contains(submission_id) {
            let message = format!("Submission {} not judged", submission_id);
            error!("{message}");
            return Err(message);
        }
    }

    Ok(())
}

fn build_initial_team_status_map(
    state: &ContestState,
) -> Result<HashMap<String, TeamStatus>, String> {
    let mut team_status_map: HashMap<String, TeamStatus> = HashMap::new();
    for team in state.teams.values() {
        let sortorder = team
            .group_ids
            .iter()
            .filter_map(|group_id| state.groups.get(group_id))
            .map(|group| group.sortorder)
            .min()
            .unwrap_or(0);

        let team_affiliation = team.organization_id.clone().ok_or_else(|| {
            let message = format!("Missing organization_id for team {}", team.id);
            error!("{message}");
            message
        })?;

        team_status_map.insert(
            team.id.clone(),
            TeamStatus::new(
                team.id.clone(),
                team.name.clone(),
                team_affiliation,
                sortorder,
            ),
        );
    }

    Ok(team_status_map)
}

fn build_judgement_order(state: &ContestState) -> Vec<&Judgement> {
    let mut judgements: Vec<&Judgement> = state.judgements.values().collect();
    judgements.sort_by(|j1, j2| {
        let s1 = state.submissions.get(&j1.submission_id);
        let s2 = state.submissions.get(&j2.submission_id);
        let s1_time = s1.and_then(|s| s.time).or(j1.start_time);
        let s2_time = s2.and_then(|s| s.time).or(j2.start_time);
        s1_time.cmp(&s2_time)
    });
    judgements
}

fn map_to_sorted_leaderboard(team_status_map: HashMap<String, TeamStatus>) -> Vec<TeamStatus> {
    let mut leaderboard: BinaryHeap<TeamStatus> = team_status_map.into_values().collect();
    let mut sorted = Vec::new();
    while let Some(team) = leaderboard.pop() {
        sorted.push(team);
    }
    sorted.reverse();
    sorted
}

fn apply_judgement_to_status(
    state: &ContestState,
    team_status_map: &mut HashMap<String, TeamStatus>,
    judgement: &Judgement,
    contest_start_time: DateTime<FixedOffset>,
) -> Result<(), String> {
    let Some(submission) = state.submissions.get(&judgement.submission_id) else {
        return Ok(());
    };

    let team_status = team_status_map
        .get_mut(&submission.team_id)
        .ok_or_else(|| {
            let message = format!("Unknown team id {}", submission.team_id);
            error!("{message}");
            message
        })?;

    let submission_time = submission.time.ok_or_else(|| {
        let message = format!("Unknown submission time for submission {}", submission.id);
        error!("{message}");
        message
    })?;

    // Freeze-specific logic is handled at processor layer by choosing which judgements to apply.
    team_status.add_submission(
        &submission.problem_id,
        submission_time,
        judgement.judgement_type_id.as_deref(),
        &state.judgement_types,
        Some(contest_start_time),
    );

    Ok(())
}

pub fn validate_and_transform(
    state: &mut ContestState,
    config: &PyriteConfig,
) -> Result<Vec<String>, String> {
    info!("Event feed parse complete, validating...");
    apply_submission_filters(state, config);

    validate_all_submissions_judged(state)?;

    let contest = state.contest.as_ref().ok_or_else(|| {
        let message = "Contest not defined".to_string();
        error!("{message}");
        message
    })?;

    let contest_start_time = contest.start_time.ok_or_else(|| {
        let message = "Contest start time not defined".to_string();
        error!("{message}");
        message
    })?;

    let contest_freeze_time = contest.scoreboard_freeze_time.ok_or_else(|| {
        let message = "Contest freeze time not defined".to_string();
        error!("{message}");
        message
    })?;

    let judgements = build_judgement_order(state);

    let mut pre_freeze_map = build_initial_team_status_map(state)?;
    let mut finalized_map = pre_freeze_map.clone();
    let mut remaining_judgements_after_freeze = Vec::new();
    let mut warnings = Vec::new();

    for judgement in judgements {
        let Some(submission) = state.submissions.get(&judgement.submission_id) else {
            let warning = format!(
                "Skipping judgement {} because submission {} is missing",
                judgement.id, judgement.submission_id
            );
            warn!("{warning}");
            warnings.push(warning);
            continue;
        };

        let submission_time = submission.time.or(judgement.start_time).ok_or_else(|| {
            let message = format!("Unknown submission time for submission {}", submission.id);
            error!("{message}");
            message
        })?;

        if submission_time > contest_freeze_time {
            remaining_judgements_after_freeze.push(judgement.clone());
        } else {
            apply_judgement_to_status(state, &mut pre_freeze_map, judgement, contest_start_time)?;
        }

        // Finalized board applies all judgements.
        apply_judgement_to_status(state, &mut finalized_map, judgement, contest_start_time)?;
    }

    state.leaderboard_pre_freeze = map_to_sorted_leaderboard(pre_freeze_map);
    state.leaderboard_finalized = map_to_sorted_leaderboard(finalized_map);
    state.remaining_judgements_after_freeze = remaining_judgements_after_freeze;

    for (rank, item) in state.leaderboard_pre_freeze.iter().enumerate() {
        info!(
            "Pre-freeze Rank {:0>3} Penalty {} TeamName: {}",
            rank + 1,
            item.total_penalty,
            item.team_name
        );
    }

    for (rank, item) in state.leaderboard_finalized.iter().enumerate() {
        info!(
            "Finalized Rank {:0>3} Penalty {} TeamName: {}",
            rank + 1,
            item.total_penalty,
            item.team_name
        );
    }

    info!(
        "Remaining judgements after freeze: {}",
        state.remaining_judgements_after_freeze.len()
    );

    Ok(warnings)
}
