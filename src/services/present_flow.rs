use std::collections::HashMap;

use tracing::{debug, warn};

use crate::models::TeamStatus;

#[derive(Clone, Default)]
pub enum SpacePhase {
    #[default]
    RevealStep,
    ApplyPostReveal {
        solved_resort: Option<(String, String)>,
        next_index: Option<usize>,
        scroll_index: Option<usize>,
    },
    ShowAward {
        team_id: String,
        citations: Vec<String>,
        next_index: Option<usize>,
        scroll_index: Option<usize>,
    },
    PendingAward {
        team_id: String,
        citations: Vec<String>,
        next_index: Option<usize>,
        scroll_index: Option<usize>,
    },
    PostAwardScroll {
        next_index: Option<usize>,
        scroll_index: Option<usize>,
    },
    Finished,
}

#[derive(Default)]
pub struct PresentFlowState {
    pub current_reveal_index: Option<usize>,
    pub reveal_initialized: bool,
    pub space_phase: SpacePhase,
}

#[derive(Default)]
pub struct AdvanceOutcome {
    pub scroll_index: Option<usize>,
    pub row_reorder: Option<(Vec<String>, Vec<String>)>,
}

pub fn current_award_payload(phase: &SpacePhase) -> Option<(&str, &[String])> {
    match phase {
        SpacePhase::ShowAward {
            team_id, citations, ..
        } => Some((team_id.as_str(), citations.as_slice())),
        _ => None,
    }
}

pub fn sync_reveal_focus_on_enter(
    flow: &mut PresentFlowState,
    board: &[TeamStatus],
) -> Option<usize> {
    if !matches!(flow.space_phase, SpacePhase::RevealStep) {
        return None;
    }

    if !flow.reveal_initialized
        || flow
            .current_reveal_index
            .is_some_and(|index| index >= board.len())
    {
        flow.current_reveal_index = find_last_pending_index(board);
        flow.reveal_initialized = true;
        return flow.current_reveal_index;
    }
    None
}

pub fn advance_space_phase(
    flow: &mut PresentFlowState,
    board: &mut Vec<TeamStatus>,
    ordered_problem_ids: &[String],
    awards_by_team: &mut HashMap<String, Vec<String>>,
) -> AdvanceOutcome {
    if board.is_empty() {
        tracing::error!("Board is empty!");
        unreachable!()
    }

    let mut outcome = AdvanceOutcome::default();
    let current_phase = std::mem::replace(&mut flow.space_phase, SpacePhase::Finished);
    flow.space_phase = match current_phase {
        SpacePhase::Finished => SpacePhase::Finished,
        SpacePhase::ShowAward {
            team_id,
            citations,
            next_index,
            scroll_index,
        } => {
            debug!(
                "Space phase: ShowAward -> PostAwardScroll(team_id={})",
                team_id
            );
            let _ = citations;
            SpacePhase::PostAwardScroll {
                next_index,
                scroll_index,
            }
        }
        SpacePhase::PendingAward {
            team_id,
            citations,
            next_index,
            scroll_index,
        } => {
            debug!(
                "Space phase: PendingAward -> ShowAward(team_id={})",
                team_id
            );
            SpacePhase::ShowAward {
                team_id,
                citations,
                next_index,
                scroll_index,
            }
        }
        SpacePhase::PostAwardScroll {
            next_index,
            scroll_index,
        } => {
            flow.current_reveal_index = next_index;
            outcome.scroll_index = clamp_scroll_index(scroll_index, board.len());
            if board.iter().any(team_has_pending_freeze) {
                debug!("Space phase: PostAwardScroll -> RevealStep");
                SpacePhase::RevealStep
            } else {
                flow.current_reveal_index = None;
                debug!("Space phase: PostAwardScroll -> Finished");
                SpacePhase::Finished
            }
        }
        SpacePhase::ApplyPostReveal {
            solved_resort,
            next_index,
            scroll_index,
        } => {
            if let Some((team_id, problem_id)) = solved_resort {
                let before_order: Vec<String> =
                    board.iter().map(|team| team.team_id.clone()).collect();
                if let Some(team) = board.iter_mut().find(|team| team.team_id == team_id) {
                    let _ = apply_solved_problem_score(team, &problem_id);
                }
                resort_leaderboard(board.as_mut_slice());
                let after_order: Vec<String> =
                    board.iter().map(|team| team.team_id.clone()).collect();
                outcome.row_reorder = Some((before_order, after_order));
            }

            flow.current_reveal_index = next_index;
            outcome.scroll_index = clamp_scroll_index(scroll_index, board.len());
            if board.iter().any(team_has_pending_freeze) {
                debug!("Space phase: ApplyPostReveal -> RevealStep");
                SpacePhase::RevealStep
            } else {
                flow.current_reveal_index = None;
                debug!("Space phase: ApplyPostReveal -> Finished");
                SpacePhase::Finished
            }
        }
        SpacePhase::RevealStep => {
            if !board.iter().any(team_has_pending_freeze) {
                flow.current_reveal_index = None;
                warn!("No more team to reveal");
                debug!("Space phase: RevealStep -> Finished");
                SpacePhase::Finished
            } else {
                if flow.current_reveal_index.is_none() {
                    flow.current_reveal_index = find_last_pending_index(board);
                    outcome.scroll_index = flow.current_reveal_index;
                    SpacePhase::RevealStep
                } else {
                    let current_index = clamp_current_index(flow.current_reveal_index, board.len());
                    let acted_team_id = board[current_index].team_id.clone();

                    if let Some(problem_id) =
                        find_next_pending_problem_id(&board[current_index], ordered_problem_ids)
                    {
                        if let Some(team) = board.get_mut(current_index)
                            && let Some(is_solved) = reveal_problem_result(team, &problem_id)
                            && is_solved
                        {
                            debug!("Space phase: RevealStep -> ApplyPostReveal(solved)");
                            SpacePhase::ApplyPostReveal {
                                solved_resort: Some((team.team_id.clone(), problem_id)),
                                next_index: Some(current_index),
                                scroll_index: Some(current_index),
                            }
                        } else if board
                            .get(current_index)
                            .is_some_and(team_has_pending_freeze)
                        {
                            flow.current_reveal_index = Some(current_index);
                            outcome.scroll_index = clamp_scroll_index(Some(current_index), board.len());
                            debug!("Space phase: RevealStep -> RevealStep(unsolved)");
                            SpacePhase::RevealStep
                        } else {
                            let (award, next_index, scroll_index) = plan_award_or_advance(
                                awards_by_team,
                                board,
                                Some(acted_team_id),
                                current_index.saturating_sub(1),
                            );
                            if let Some((team_id, citations)) = award {
                                debug!("Space phase: RevealStep -> PendingAward(team_id={})", team_id);
                                SpacePhase::PendingAward {
                                    team_id,
                                    citations,
                                    next_index,
                                    scroll_index,
                                }
                            } else {
                                debug!("Space phase: RevealStep -> ApplyPostReveal(advance)");
                                SpacePhase::ApplyPostReveal {
                                    solved_resort: None,
                                    next_index,
                                    scroll_index,
                                }
                            }
                        }
                    } else {
                        let (award, next_index, scroll_index) = plan_award_or_advance(
                            awards_by_team,
                            board,
                            Some(acted_team_id),
                            current_index.saturating_sub(1),
                        );
                        if let Some((team_id, citations)) = award {
                            debug!("Space phase: RevealStep -> ShowAward(team_id={})", team_id);
                            SpacePhase::ShowAward {
                                team_id,
                                citations,
                                next_index,
                                scroll_index,
                            }
                        } else {
                            flow.current_reveal_index = next_index;
                            outcome.scroll_index = clamp_scroll_index(scroll_index, board.len());
                            if board.iter().any(team_has_pending_freeze) {
                                debug!("Space phase: RevealStep -> RevealStep(advance)");
                                SpacePhase::RevealStep
                            } else {
                                flow.current_reveal_index = None;
                                debug!("Space phase: RevealStep -> Finished(advance)");
                                SpacePhase::Finished
                            }
                        }
                    }
                }
            }
        }
    };

    outcome
}

fn team_has_pending_freeze(team: &TeamStatus) -> bool {
    team.problem_stats
        .values()
        .any(|stat| stat.attempted_during_freeze)
}

fn maybe_take_award_for_team(
    awards_by_team: &mut HashMap<String, Vec<String>>,
    team: &TeamStatus,
) -> Option<Vec<String>> {
    if team_has_pending_freeze(team) {
        return None;
    }
    awards_by_team.remove(&team.team_id)
}

fn find_last_pending_index(board: &[TeamStatus]) -> Option<usize> {
    board.iter().rposition(team_has_pending_freeze)
}

fn find_next_pending_problem_id(team: &TeamStatus, ordered_problem_ids: &[String]) -> Option<String> {
    for problem_id in ordered_problem_ids {
        if team
            .problem_stats
            .get(problem_id)
            .is_some_and(|stat| stat.attempted_during_freeze)
        {
            return Some(problem_id.clone());
        }
    }

    team.problem_stats
        .iter()
        .find(|(_, stat)| stat.attempted_during_freeze)
        .map(|(problem_id, _)| problem_id.clone())
}

fn reveal_problem_result(team: &mut TeamStatus, problem_id: &str) -> Option<bool> {
    let problem_stat = team.problem_stats.get_mut(problem_id)?;
    if !problem_stat.attempted_during_freeze {
        return None;
    }

    problem_stat.attempted_during_freeze = false;
    Some(problem_stat.solved)
}

fn apply_solved_problem_score(team: &mut TeamStatus, problem_id: &str) -> bool {
    let Some(problem_stat) = team.problem_stats.get(problem_id) else {
        return false;
    };
    if !problem_stat.solved {
        return false;
    }

    team.total_points += 1;
    team.total_penalty += problem_stat.penalty;
    if let Some(ac_time) = problem_stat.first_ac_time
        && team
            .last_ac_time
            .is_none_or(|last_time| ac_time > last_time)
    {
        team.last_ac_time = Some(ac_time);
    }

    true
}

fn resort_leaderboard(board: &mut [TeamStatus]) {
    board.sort();
}

fn plan_award_or_advance(
    awards_by_team: &mut HashMap<String, Vec<String>>,
    board: &[TeamStatus],
    acted_team_id: Option<String>,
    next_index: usize,
) -> (Option<(String, Vec<String>)>, Option<usize>, Option<usize>) {
    let has_pending = board.iter().any(team_has_pending_freeze);
    if let Some(team_id) = acted_team_id
        && let Some(team) = board.iter().find(|team| team.team_id == team_id)
        && let Some(citations) = maybe_take_award_for_team(awards_by_team, team)
    {
        let (next, scroll) = if has_pending {
            (Some(next_index), Some(next_index))
        } else {
            (None, None)
        };
        return (Some((team.team_id.clone(), citations)), next, scroll);
    }
    let (next, scroll) = if has_pending {
        (Some(next_index), Some(next_index))
    } else {
        (None, None)
    };
    (None, next, scroll)
}

fn clamp_current_index(current: Option<usize>, len: usize) -> usize {
    current.unwrap_or_default().min(len.saturating_sub(1))
}

fn clamp_scroll_index(index: Option<usize>, len: usize) -> Option<usize> {
    index.map(|i| i.min(len.saturating_sub(1)))
}
