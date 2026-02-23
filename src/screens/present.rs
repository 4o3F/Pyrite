use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;

use crate::models::{ContestState, Problem, TeamStatus};
use crate::services::config_loader::PyriteConfig;

pub enum PresentAction {
    Stay,
}

#[derive(Default)]
struct PresentUiState {
    /// Scroll offset in points for the scoreboard viewport.
    viewpoint_offset: f32,
    current_reveal_index: Option<usize>,
    reveal_initialized: bool,
    logo_cache: HashMap<String, Option<egui::TextureHandle>>,
}

#[derive(Clone)]
struct FrameMetrics {
    row_height: f32,
    header_height: f32,
    outer_pad_x: f32,
    inner_pad_y: f32,
    col_gap: f32,
    logo_size: f32,
    rank_font: egui::FontId,
    team_font: egui::FontId,
    problem_font: egui::FontId,
    stat_font: egui::FontId,
    header_font: egui::FontId,
    rank_col_width: f32,
    solved_col_width: f32,
    time_col_width: f32,
}

#[derive(Clone, Copy)]
struct RowLayout {
    rank_rect: egui::Rect,
    logo_rect: egui::Rect,
    center_rect: egui::Rect,
    solved_rect: egui::Rect,
    time_rect: egui::Rect,
}

thread_local! {
    static PRESENT_UI_STATE: RefCell<PresentUiState> = RefCell::new(PresentUiState::default());
}

pub fn ui(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    contest_state: &mut ContestState,
    data_path: Option<&str>,
    config: &PyriteConfig,
) -> PresentAction {
    PRESENT_UI_STATE.with(|cell| {
        let mut state = cell.borrow_mut();

        let metrics = compute_frame_metrics(
            ui.painter(),
            ui.available_height(),
            ui.available_width(),
            config.presentation.rows_per_page.max(1),
            contest_state,
        );

        let even_row_bg = egui::Color32::from_gray(32);
        let odd_row_bg = egui::Color32::from_gray(12);
        let focused_row_bg = egui::Color32::from_rgb(116, 212, 255);
        let solved_bg = egui::Color32::from_rgb(49, 201, 80);
        let attempted_bg = egui::Color32::from_rgb(251, 44, 54);
        let attempted_freeze_bg = egui::Color32::from_rgb(43, 127, 255);
        let untouched_bg = egui::Color32::from_rgb(98, 116, 142);

        let mut problems: Vec<Problem> = contest_state.problems.values().cloned().collect();
        problems.sort_by(|a, b| a.ordinal.cmp(&b.ordinal).then(a.label.cmp(&b.label)));
        let ordered_problem_ids: Vec<String> =
            problems.iter().map(|problem| problem.id.clone()).collect();
        let row_count = contest_state.leaderboard_pre_freeze.len();

        // Header row
        let (header_rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), metrics.header_height),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(header_rect, 0.0, egui::Color32::from_gray(20));
        let header_layout = compute_row_layout(header_rect, &metrics);
        ui.painter().text(
            egui::pos2(
                header_layout.rank_rect.center().x,
                header_layout.rank_rect.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            "Rank",
            metrics.header_font.clone(),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(
                header_layout.center_rect.center().x,
                header_layout.center_rect.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            "Team / Problems",
            metrics.header_font.clone(),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(
                header_layout.solved_rect.center().x,
                header_layout.solved_rect.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            "Solved",
            metrics.header_font.clone(),
            egui::Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(
                header_layout.time_rect.center().x,
                header_layout.time_rect.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            "Time",
            metrics.header_font.clone(),
            egui::Color32::WHITE,
        );
        ui.add_space(4.0);

        let scroll_height = (ui.available_height()).max(80.0);
        sync_reveal_focus_on_enter(&mut state, contest_state, &metrics, scroll_height);
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            handle_space_step(
                &mut state,
                contest_state,
                &ordered_problem_ids,
                metrics.row_height,
                scroll_height,
            );
        }

        let content_height = row_count as f32 * metrics.row_height;

        egui::ScrollArea::vertical()
            .id_salt("present_pre_freeze_scroll")
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .vertical_scroll_offset(state.viewpoint_offset)
            .show_viewport(ui, |ui, viewport| {
                if row_count == 0 {
                    ui.label("No teams in pre-freeze leaderboard.");
                    return;
                }

                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), content_height.max(viewport.height())),
                    egui::Sense::hover(),
                );

                let start_row = (viewport.min.y / metrics.row_height).floor().max(0.0) as usize;
                let end_row = ((viewport.max.y / metrics.row_height).ceil() as usize)
                    .min(row_count.saturating_sub(1));

                for idx in start_row..=end_row {
                    let team = &contest_state.leaderboard_pre_freeze[idx];
                    let row_top = rect.top() + idx as f32 * metrics.row_height;
                    let row_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), row_top),
                        egui::vec2(rect.width(), metrics.row_height),
                    );
                    let layout = compute_row_layout(row_rect, &metrics);

                    let bg = if state.current_reveal_index == Some(idx) {
                        focused_row_bg
                    } else if idx % 2 == 0 {
                        even_row_bg
                    } else {
                        odd_row_bg
                    };
                    ui.painter().rect_filled(row_rect, 0.0, bg);

                    // Debug: visualize left / center / right zones.
                    // let left_rect = egui::Rect::from_min_max(
                    //     egui::pos2(layout.rank_rect.left(), row_rect.top()),
                    //     egui::pos2(layout.logo_rect.right(), row_rect.bottom()),
                    // );
                    // let right_rect = egui::Rect::from_min_max(
                    //     egui::pos2(layout.solved_rect.left(), row_rect.top()),
                    //     egui::pos2(layout.time_rect.right(), row_rect.bottom()),
                    // );
                    // ui.painter().rect_stroke(
                    //     left_rect,
                    //     0.0,
                    //     egui::Stroke::new(1.0, egui::Color32::YELLOW),
                    //     egui::StrokeKind::Inside,
                    // );
                    // ui.painter().rect_stroke(
                    //     layout.center_rect,
                    //     0.0,
                    //     egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                    //     egui::StrokeKind::Inside,
                    // );
                    // ui.painter().rect_stroke(
                    //     right_rect,
                    //     0.0,
                    //     egui::Stroke::new(1.0, egui::Color32::LIGHT_RED),
                    //     egui::StrokeKind::Inside,
                    // );

                    render_left_zone(
                        ui,
                        &mut state,
                        ctx,
                        contest_state,
                        team,
                        idx + 1,
                        data_path,
                        config,
                        &layout,
                        &metrics,
                    );
                    render_center_zone(
                        ui,
                        team,
                        &problems,
                        &layout,
                        &metrics,
                        solved_bg,
                        attempted_bg,
                        attempted_freeze_bg,
                        untouched_bg,
                    );
                    render_right_zone(ui, team, &layout, &metrics);
                }
            });
    });

    PresentAction::Stay
}

fn team_has_pending_freeze(team: &TeamStatus) -> bool {
    team.problem_stats
        .values()
        .any(|stat| stat.attempted_during_freeze)
}

fn find_last_pending_index(board: &[TeamStatus]) -> Option<usize> {
    board.iter().rposition(team_has_pending_freeze)
}

fn find_next_pending_problem_id(
    team: &TeamStatus,
    ordered_problem_ids: &[String],
) -> Option<String> {
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

fn apply_reveal_for_problem(team: &mut TeamStatus, problem_id: &str) -> bool {
    let Some(problem_stat) = team.problem_stats.get_mut(problem_id) else {
        return false;
    };
    if !problem_stat.attempted_during_freeze {
        return false;
    }

    problem_stat.attempted_during_freeze = false;
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

fn row_offset_for_index(
    index: usize,
    row_height: f32,
    viewport_height: f32,
    row_count: usize,
) -> f32 {
    let target = index as f32 * row_height - viewport_height * (2.0 / 3.0);
    let max_offset = (row_count as f32 * row_height - viewport_height).max(0.0);
    target.clamp(0.0, max_offset)
}

fn sync_reveal_focus_on_enter(
    state: &mut PresentUiState,
    contest_state: &ContestState,
    metrics: &FrameMetrics,
    viewport_height: f32,
) {
    let board = &contest_state.leaderboard_pre_freeze;
    if !state.reveal_initialized
        || state
            .current_reveal_index
            .is_some_and(|index| index >= board.len())
    {
        state.current_reveal_index = find_last_pending_index(board);
        state.reveal_initialized = true;
        if let Some(index) = state.current_reveal_index {
            state.viewpoint_offset =
                row_offset_for_index(index, metrics.row_height, viewport_height, board.len());
        }
    }
}

fn handle_space_step(
    state: &mut PresentUiState,
    contest_state: &mut ContestState,
    ordered_problem_ids: &[String],
    row_height: f32,
    viewport_height: f32,
) {
    let board = &mut contest_state.leaderboard_pre_freeze;
    if board.is_empty() {
        tracing::error!("Board is empty!");
        unreachable!()
    }

    if !board.iter().any(team_has_pending_freeze) {
        state.current_reveal_index = None;
        tracing::warn!("No more team to reveal");
        return;
    }

    if state.current_reveal_index.is_none() {
        state.current_reveal_index = find_last_pending_index(board);
        if let Some(index) = state.current_reveal_index {
            state.viewpoint_offset =
                row_offset_for_index(index, row_height, viewport_height, board.len());
        }
        return;
    }

    let mut index = state.current_reveal_index.unwrap_or_default();
    if index >= board.len() {
        index = board.len().saturating_sub(1);
    }

    if let Some(problem_id) = find_next_pending_problem_id(&board[index], ordered_problem_ids) {
        if let Some(team) = board.get_mut(index) {
            let _ = apply_reveal_for_problem(team, &problem_id);
        }

        resort_leaderboard(board.as_mut_slice());
        index = index.min(board.len().saturating_sub(1));
    } else {
        index = index.saturating_sub(1);
    }

    state.current_reveal_index = Some(index);
    state.viewpoint_offset = row_offset_for_index(index, row_height, viewport_height, board.len());

    if !board.iter().any(team_has_pending_freeze) {
        state.current_reveal_index = None;
    }
}

fn compute_frame_metrics(
    painter: &egui::Painter,
    viewport_height: f32,
    viewport_width: f32,
    rows_per_page: usize,
    contest_state: &ContestState,
) -> FrameMetrics {
    let row_height = viewport_height / rows_per_page as f32;
    let header_height = row_height * 0.5;
    let outer_pad_x = viewport_width * 0.008;
    let inner_pad_y = row_height * 0.08;
    let col_gap = viewport_width * 0.006;
    let logo_size = (row_height - inner_pad_y * 2.0).max(18.0);

    let rank_font = egui::FontId::proportional(row_height * 0.45);
    let team_font = egui::FontId::proportional(row_height * 0.34);
    let problem_font = egui::FontId::proportional(row_height * 0.3);
    let stat_font = egui::FontId::proportional(row_height * 0.45);
    let header_font = egui::FontId::proportional(row_height * 0.28);

    let rank_digits = contest_state.teams.len().to_string().len();
    let rank_sample = "0".repeat(rank_digits);
    let rank_col_width = text_width(painter, &rank_sample, &rank_font).max(text_width(
        painter,
        "Rank",
        &header_font,
    ));

    let max_solved = contest_state.problems.len();
    let max_time = contest_state
        .leaderboard_finalized
        .iter()
        .map(|t| t.total_penalty.to_string())
        .max_by_key(String::len)
        .unwrap_or_else(|| "0".to_string());

    let solved_col_width = text_width(painter, "Solved", &header_font).max(text_width(
        painter,
        &max_solved.to_string(),
        &stat_font,
    )) + col_gap * 0.8;
    let time_col_width = text_width(painter, "Time", &header_font)
        .max(text_width(painter, &max_time, &stat_font))
        + col_gap * 0.8;

    FrameMetrics {
        row_height,
        header_height,
        outer_pad_x,
        inner_pad_y,
        col_gap,
        logo_size,
        rank_font,
        team_font,
        problem_font,
        stat_font,
        header_font,
        rank_col_width,
        solved_col_width,
        time_col_width,
    }
}

fn compute_row_layout(row_rect: egui::Rect, m: &FrameMetrics) -> RowLayout {
    let inner = egui::Rect::from_min_max(
        egui::pos2(
            row_rect.left() + m.outer_pad_x,
            row_rect.top() + m.inner_pad_y,
        ),
        egui::pos2(
            row_rect.right() - m.outer_pad_x,
            row_rect.bottom() - m.inner_pad_y,
        ),
    );

    let time_rect = egui::Rect::from_min_size(
        egui::pos2(inner.right() - m.time_col_width, inner.top()),
        egui::vec2(m.time_col_width, inner.height()),
    );
    let solved_rect = egui::Rect::from_min_size(
        egui::pos2(
            time_rect.left() - m.col_gap - m.solved_col_width,
            inner.top(),
        ),
        egui::vec2(m.solved_col_width, inner.height()),
    );

    let rank_rect = egui::Rect::from_min_size(
        egui::pos2(inner.left(), inner.top()),
        egui::vec2(m.rank_col_width, inner.height()),
    );
    let logo_rect = egui::Rect::from_center_size(
        egui::pos2(
            rank_rect.right() + m.col_gap + m.logo_size * 0.5,
            inner.center().y,
        ),
        egui::vec2(m.logo_size, m.logo_size),
    );

    let center_left = logo_rect.right() + m.col_gap;
    let center_right = (solved_rect.left() - m.col_gap).max(center_left);
    let center_rect = egui::Rect::from_min_max(
        egui::pos2(center_left, inner.top()),
        egui::pos2(center_right, inner.bottom()),
    );

    RowLayout {
        rank_rect,
        logo_rect,
        center_rect,
        solved_rect,
        time_rect,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_left_zone(
    ui: &mut egui::Ui,
    state: &mut PresentUiState,
    ctx: &egui::Context,
    contest_state: &ContestState,
    team: &TeamStatus,
    rank: usize,
    data_path: Option<&str>,
    config: &PyriteConfig,
    layout: &RowLayout,
    m: &FrameMetrics,
) {
    ui.painter().text(
        egui::pos2(layout.rank_rect.center().x, layout.rank_rect.center().y),
        egui::Align2::CENTER_CENTER,
        format!("{rank}"),
        m.rank_font.clone(),
        egui::Color32::WHITE,
    );

    if let Some(texture) =
        ensure_logo_loaded(state, ctx, contest_state, &team.team_id, data_path, config)
    {
        let image = egui::Image::new(&texture)
            .fit_to_exact_size(layout.logo_rect.size())
            .corner_radius(egui::CornerRadius::same(
                (layout.logo_rect.height() * 0.5) as u8,
            ));
        ui.put(layout.logo_rect, image);
    } else {
        ui.painter().circle_filled(
            layout.logo_rect.center(),
            layout.logo_rect.height() * 0.5,
            egui::Color32::from_gray(72),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_center_zone(
    ui: &mut egui::Ui,
    team: &TeamStatus,
    problems: &[Problem],
    layout: &RowLayout,
    m: &FrameMetrics,
    solved_bg: egui::Color32,
    attempted_bg: egui::Color32,
    attempted_freeze_bg: egui::Color32,
    untouched_bg: egui::Color32,
) {
    let name_y = layout.center_rect.top();
    let status_y = layout.center_rect.bottom() - layout.center_rect.height() * 0.4;

    let name_rect = egui::Rect::from_min_max(
        egui::pos2(layout.center_rect.left(), layout.center_rect.top()),
        egui::pos2(
            layout.center_rect.right(),
            layout.center_rect.top() + layout.center_rect.height() * 0.52,
        ),
    );
    ui.painter().with_clip_rect(name_rect).text(
        egui::pos2(layout.center_rect.left(), name_y),
        egui::Align2::LEFT_TOP,
        &team.team_name,
        m.team_font.clone(),
        egui::Color32::WHITE,
    );

    if problems.is_empty() {
        unreachable!()
    }

    let n = problems.len() as f32;
    let cell_gap = (layout.center_rect.width() * 0.006).max(10.);
    let cell_width = (layout.center_rect.width() - cell_gap * (n - 1.0)) / n;
    let cell_height = layout.center_rect.height() * 0.4;
    // tracing::debug!("{} {} {} {}", cell_width, cell_height, layout.center_rect.width(), cell_gap);
    let mut cell_x = layout.center_rect.left();

    for problem in problems {
        let stat = team.problem_stats.get(problem.id.as_str());
        let fill = match stat {
            Some(s) if s.attempted_during_freeze => attempted_freeze_bg,
            Some(s) if s.solved => solved_bg,
            Some(s) if s.submissions_before_solved > 0 => attempted_bg,
            _ => untouched_bg,
        };
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(cell_x, status_y),
            egui::vec2(cell_width, cell_height),
        );
        ui.painter().rect_filled(status_rect, 2.0, fill);
        ui.painter().text(
            status_rect.center(),
            egui::Align2::CENTER_CENTER,
            &problem.label,
            m.problem_font.clone(),
            egui::Color32::WHITE,
        );
        cell_x += cell_width + cell_gap;
    }
}

fn render_right_zone(ui: &mut egui::Ui, team: &TeamStatus, layout: &RowLayout, m: &FrameMetrics) {
    ui.painter().text(
        egui::pos2(layout.solved_rect.center().x, layout.solved_rect.center().y),
        egui::Align2::CENTER_CENTER,
        team.total_points.to_string(),
        m.stat_font.clone(),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(layout.time_rect.center().x, layout.time_rect.center().y),
        egui::Align2::CENTER_CENTER,
        team.total_penalty.to_string(),
        m.stat_font.clone(),
        egui::Color32::WHITE,
    );
}

fn text_width(painter: &egui::Painter, text: &str, font: &egui::FontId) -> f32 {
    painter
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x
}

fn ensure_logo_loaded(
    state: &mut PresentUiState,
    ctx: &egui::Context,
    contest_state: &ContestState,
    team_id: &str,
    data_path: Option<&str>,
    config: &PyriteConfig,
) -> Option<egui::TextureHandle> {
    if let Some(cached) = state.logo_cache.get(team_id) {
        return cached.clone();
    }

    let loaded = resolve_team_logo_path(contest_state, team_id, data_path, config)
        .and_then(|path| load_logo_texture(ctx, team_id, &path));
    state.logo_cache.insert(team_id.to_string(), loaded.clone());
    loaded
}

fn resolve_team_logo_path(
    contest_state: &ContestState,
    team_id: &str,
    data_path: Option<&str>,
    config: &PyriteConfig,
) -> Option<PathBuf> {
    let base = PathBuf::from(data_path?);
    let team = contest_state.teams.get(team_id)?;
    let org_id = team.organization_id.as_ref()?;

    // Require organization to exist in parsed state, but file naming is fixed by org_id + config extension.
    let _org = contest_state.organizations.get(org_id)?;

    let ext = config
        .presentation
        .logo_extension
        .trim()
        .trim_start_matches('.');
    if ext.is_empty() {
        return None;
    }
    let file_name = format!("{org_id}.{ext}");

    let file_path = base.join("affiliations").join(&file_name);
    if file_path.exists() && file_path.is_file() {
        Some(file_path)
    } else {
        None
    }
}

fn load_logo_texture(
    ctx: &egui::Context,
    team_id: &str,
    path: &Path,
) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(
        format!("team_logo_{team_id}"),
        image,
        egui::TextureOptions::LINEAR,
    ))
}
