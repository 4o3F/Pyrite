use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use eframe::egui;
use image::GenericImageView;
use tracing::{debug, info, warn};

use crate::models::{ContestState, Problem, TeamStatus};
use crate::services::config_loader::PyriteConfig;
use crate::services::present_flow::{self, PresentFlowState};

pub enum PresentAction {
    Stay,
}

#[derive(Default)]
struct PresentUiState {
    scroll_current_offset: f32,
    scroll_target_offset: f32,
    scroll_anim_start_offset: f32,
    scroll_anim_start_time: Option<f64>,
    scroll_anim_duration: f32,
    flow: PresentFlowState,
    active_row_anims: HashMap<String, RowMoveAnim>,
    logo_cache: HashMap<String, Option<egui::TextureHandle>>,
    award_photo_cache: HashMap<String, Option<egui::TextureHandle>>,
    award_fallback_texture: Option<Option<egui::TextureHandle>>,
    awards_initialized: bool,
    awards_by_team: HashMap<String, Vec<String>>,
    award_decode_started: bool,
    award_decode_rx: Option<Receiver<AwardDecodeMsg>>,
    decoded_award_images: HashMap<String, Option<DecodedImageData>>,
    decoded_award_fallback: Option<Option<DecodedImageData>>,
}

struct DecodedImageData {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

enum AwardDecodeMsg {
    Team {
        team_id: String,
        image: Option<DecodedImageData>,
    },
    Fallback(Option<DecodedImageData>),
}

#[derive(Clone, Copy)]
struct RowMoveAnim {
    from_index: usize,
    to_index: usize,
    started_at: f64,
    duration_sec: f32,
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
        let now = now_seconds(ctx);
        let scroll_duration = config.presentation.scroll_animation_seconds.max(0.01);
        let row_fly_seconds_per_row = config.presentation.row_fly_animation_seconds.max(0.01);
        state.scroll_anim_duration = scroll_duration;

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
        ensure_awards_initialized(&mut state, contest_state);
        maybe_start_award_predecode(&mut state, contest_state, data_path, config);
        pump_award_predecode(&mut state);

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
        if let Some(index) = present_flow::sync_reveal_focus_on_enter(
            &mut state.flow,
            &contest_state.leaderboard_pre_freeze,
        ) {
            set_scroll_target_for_index(
                &mut state,
                index,
                metrics.row_height,
                scroll_height,
                row_count,
                now,
                false,
            );
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            let mut awards_by_team = std::mem::take(&mut state.awards_by_team);
            let outcome = present_flow::advance_space_phase(
                &mut state.flow,
                &mut contest_state.leaderboard_pre_freeze,
                &ordered_problem_ids,
                &mut awards_by_team,
            );
            state.awards_by_team = awards_by_team;
            if let Some((before_order, after_order)) = outcome.row_reorder {
                spawn_row_move_animations(
                    &mut state,
                    &before_order,
                    &after_order,
                    now,
                    row_fly_seconds_per_row,
                );
            }
            if let Some(index) = outcome.scroll_index {
                set_scroll_target_for_index(
                    &mut state,
                    index,
                    metrics.row_height,
                    scroll_height,
                    contest_state.leaderboard_pre_freeze.len(),
                    now,
                    true,
                );
            }
        }
        let scroll_animating = update_scroll_animation(&mut state, now);
        let row_animating = cleanup_and_has_active_row_anims(&mut state, now);

        let content_height = row_count as f32 * metrics.row_height;

        egui::ScrollArea::vertical()
            .id_salt("present_pre_freeze_scroll")
            .auto_shrink([false, false])
            .max_height(scroll_height)
            .vertical_scroll_offset(state.scroll_current_offset)
            .show_viewport(ui, |ui, viewport| {
                if row_count == 0 {
                    ui.label("No teams in pre-freeze leaderboard.");
                    return;
                }

                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), content_height.max(viewport.height())),
                    egui::Sense::hover(),
                );

                let mut draw_rows: Vec<(usize, f32, bool)> = (0..row_count)
                    .map(|idx| {
                        let team_id = contest_state.leaderboard_pre_freeze[idx].team_id.as_str();
                        let animated_y =
                            row_content_y_for_team(&state, team_id, idx, metrics.row_height, now);
                        let rising_top_layer = is_rising_row_anim_active(&state, team_id, now);
                        (idx, animated_y, rising_top_layer)
                    })
                    .filter(|(_, row_y, _)| {
                        let row_min = *row_y;
                        let row_max = row_min + metrics.row_height;
                        row_max >= viewport.min.y && row_min <= viewport.max.y
                    })
                    .collect();
                draw_rows.sort_by(|a, b| a.1.total_cmp(&b.1));

                for (idx, row_y, rising_top_layer) in draw_rows.iter().copied() {
                    if rising_top_layer {
                        continue;
                    }
                    let team = &contest_state.leaderboard_pre_freeze[idx];
                    let row_top = rect.top() + row_y;
                    let row_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), row_top),
                        egui::vec2(rect.width(), metrics.row_height),
                    );
                    let layout = compute_row_layout(row_rect, &metrics);

                    let bg = if state.flow.current_reveal_index == Some(idx) {
                        focused_row_bg
                    } else if idx % 2 == 0 {
                        even_row_bg
                    } else {
                        odd_row_bg
                    };
                    ui.painter().rect_filled(row_rect, 0.0, bg);

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

                for (idx, row_y, rising_top_layer) in draw_rows {
                    if !rising_top_layer {
                        continue;
                    }
                    let team = &contest_state.leaderboard_pre_freeze[idx];
                    let row_top = rect.top() + row_y;
                    let row_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), row_top),
                        egui::vec2(rect.width(), metrics.row_height),
                    );
                    let layout = compute_row_layout(row_rect, &metrics);

                    let bg = if state.flow.current_reveal_index == Some(idx) {
                        focused_row_bg
                    } else if idx % 2 == 0 {
                        even_row_bg
                    } else {
                        odd_row_bg
                    };
                    ui.painter().rect_filled(row_rect, 0.0, bg);

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

        render_active_award_overlay(ui, &mut state, ctx, contest_state, data_path, config);

        if scroll_animating
            || row_animating
            || present_flow::current_award_payload(&state.flow.space_phase).is_some()
        {
            ctx.request_repaint();
        }
    });

    PresentAction::Stay
}

fn now_seconds(ctx: &egui::Context) -> f64 {
    ctx.input(|input| input.time)
}

fn anim_progress(now: f64, started_at: f64, duration_sec: f32) -> f32 {
    if duration_sec <= 0.0 {
        return 1.0;
    }
    ((now - started_at) / f64::from(duration_sec)).clamp(0.0, 1.0) as f32
}

fn ease_in_out_sine(t: f32) -> f32 {
    -(f32::cos(std::f32::consts::PI * t) - 1.0) * 0.5
}

fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

fn lerp_f32(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn ensure_awards_initialized(state: &mut PresentUiState, contest_state: &ContestState) {
    if state.awards_initialized {
        return;
    }

    let mut awards: Vec<_> = contest_state.awards.values().collect();
    awards.sort_by(|a, b| a.id.cmp(&b.id));
    for award in awards {
        let citation = award.citation.trim();
        if citation.is_empty() {
            continue;
        }
        for team_id in &award.team_ids {
            state
                .awards_by_team
                .entry(team_id.clone())
                .or_default()
                .push(citation.to_string());
        }
    }
    state.awards_initialized = true;
}

fn maybe_start_award_predecode(
    state: &mut PresentUiState,
    contest_state: &ContestState,
    data_path: Option<&str>,
    config: &PyriteConfig,
) {
    if state.award_decode_started {
        return;
    }
    state.award_decode_started = true;

    let Some(base_path) = data_path.map(PathBuf::from) else {
        warn!("Award predecode skipped: data_path is missing");
        return;
    };
    let ext = config
        .presentation
        .team_photo_extension
        .trim()
        .trim_start_matches('.')
        .to_string();
    if ext.is_empty() {
        warn!("Award predecode skipped: team_photo_extension is empty");
        return;
    }

    let fallback_path = resolve_award_fallback_path(config);
    let mut team_ids: Vec<String> = state.awards_by_team.keys().cloned().collect();
    team_ids.sort_by(|a, b| {
        contest_state
            .leaderboard_finalized
            .binary_search_by(|x| x.team_id.cmp(a))
            .cmp(
                &contest_state
                    .leaderboard_finalized
                    .binary_search_by(|x| x.team_id.cmp(b)),
            )
    });
    team_ids.reverse();
    info!(
        "Starting award image predecode for {} team(s), ext={}, fallback={}",
        team_ids.len(),
        ext,
        fallback_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    );
    let (tx, rx) = mpsc::channel::<AwardDecodeMsg>();
    state.award_decode_rx = Some(rx);

    thread::spawn(move || {
        let mut ok_count = 0usize;
        let mut miss_count = 0usize;
        let fallback_image = fallback_path
            .as_deref()
            .and_then(|path| decode_award_image_data(path, 1920));
        if fallback_image.is_some() {
            info!("Award fallback image decoded successfully");
        } else {
            debug!("Award fallback image not available or decode failed");
        }
        let _ = tx.send(AwardDecodeMsg::Fallback(fallback_image));

        for team_id in team_ids {
            let path = base_path.join("teams").join(format!("{team_id}.{ext}"));
            let image = if path.exists() && path.is_file() {
                decode_award_image_data(&path, 1920)
            } else {
                None
            };
            if image.is_some() {
                ok_count += 1;
                info!("Award image for team {} predecode finished", team_id);
            } else {
                miss_count += 1;
            }
            let _ = tx.send(AwardDecodeMsg::Team { team_id, image });
        }
        info!(
            "Award image predecode finished: ok={}, missing_or_failed={}",
            ok_count, miss_count
        );
    });
}

fn pump_award_predecode(state: &mut PresentUiState) {
    let Some(rx) = state.award_decode_rx.as_ref() else {
        return;
    };

    loop {
        match rx.try_recv() {
            Ok(AwardDecodeMsg::Team { team_id, image }) => {
                state.decoded_award_images.insert(team_id, image);
            }
            Ok(AwardDecodeMsg::Fallback(image)) => {
                state.decoded_award_fallback = Some(image);
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                info!("Award image predecode channel closed");
                state.award_decode_rx = None;
                break;
            }
        }
    }
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

fn set_scroll_target_for_index(
    state: &mut PresentUiState,
    index: usize,
    row_height: f32,
    viewport_height: f32,
    row_count: usize,
    now: f64,
    animate: bool,
) {
    let target = row_offset_for_index(index, row_height, viewport_height, row_count);
    if !animate {
        state.scroll_current_offset = target;
        state.scroll_target_offset = target;
        state.scroll_anim_start_offset = target;
        state.scroll_anim_start_time = None;
        return;
    }

    state.scroll_anim_start_offset = state.scroll_current_offset;
    state.scroll_target_offset = target;
    state.scroll_anim_start_time = Some(now);
}

fn update_scroll_animation(state: &mut PresentUiState, now: f64) -> bool {
    let Some(started_at) = state.scroll_anim_start_time else {
        return false;
    };

    let progress = anim_progress(now, started_at, state.scroll_anim_duration);
    let eased = ease_in_out_sine(progress);
    state.scroll_current_offset = lerp_f32(
        state.scroll_anim_start_offset,
        state.scroll_target_offset,
        eased,
    );

    if progress >= 1.0 {
        state.scroll_current_offset = state.scroll_target_offset;
        state.scroll_anim_start_time = None;
        return false;
    }

    true
}

fn spawn_row_move_animations(
    state: &mut PresentUiState,
    before_order: &[String],
    after_order: &[String],
    now: f64,
    seconds_per_row: f32,
) {
    let mut before_map = HashMap::with_capacity(before_order.len());
    for (idx, team_id) in before_order.iter().enumerate() {
        before_map.insert(team_id.as_str(), idx);
    }

    for (new_index, team_id) in after_order.iter().enumerate() {
        let Some(old_index) = before_map.get(team_id.as_str()).copied() else {
            continue;
        };
        if old_index == new_index {
            continue;
        }
        let duration_sec = row_move_duration_seconds(old_index, new_index, seconds_per_row);
        state.active_row_anims.insert(
            team_id.clone(),
            RowMoveAnim {
                from_index: old_index,
                to_index: new_index,
                started_at: now,
                duration_sec,
            },
        );
    }
}

fn row_move_duration_seconds(from_index: usize, to_index: usize, seconds_per_row: f32) -> f32 {
    let distance_rows = from_index.abs_diff(to_index) as f32;
    (distance_rows * seconds_per_row).max(0.01)
}

fn row_content_y_for_team(
    state: &PresentUiState,
    team_id: &str,
    logical_index: usize,
    row_height: f32,
    now: f64,
) -> f32 {
    let Some(anim) = state.active_row_anims.get(team_id) else {
        return logical_index as f32 * row_height;
    };

    let progress = anim_progress(now, anim.started_at, anim.duration_sec);
    let from_y = anim.from_index as f32 * row_height;
    let to_y = anim.to_index as f32 * row_height;
    if progress >= 1.0 {
        return to_y;
    }

    lerp_f32(from_y, to_y, ease_out_cubic(progress))
}

fn is_rising_row_anim_active(state: &PresentUiState, team_id: &str, now: f64) -> bool {
    let Some(anim) = state.active_row_anims.get(team_id) else {
        return false;
    };
    anim.to_index < anim.from_index && anim_progress(now, anim.started_at, anim.duration_sec) < 1.0
}

fn cleanup_and_has_active_row_anims(state: &mut PresentUiState, now: f64) -> bool {
    state
        .active_row_anims
        .retain(|_, anim| anim_progress(now, anim.started_at, anim.duration_sec) < 1.0);
    !state.active_row_anims.is_empty()
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
        let cell_text = match stat {
            Some(s) if s.submissions_before_solved > 0 => &format!(
                "{} - {}",
                s.submissions_before_solved, s.last_submission_time
            ),
            Some(_) | None => &problem.label,
        };
        let status_rect = egui::Rect::from_min_size(
            egui::pos2(cell_x, status_y),
            egui::vec2(cell_width, cell_height),
        );
        ui.painter().rect_filled(status_rect, 2.0, fill);
        ui.painter().text(
            status_rect.center(),
            egui::Align2::CENTER_CENTER,
            cell_text,
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

fn render_active_award_overlay(
    ui: &mut egui::Ui,
    state: &mut PresentUiState,
    ctx: &egui::Context,
    contest_state: &ContestState,
    data_path: Option<&str>,
    config: &PyriteConfig,
) {
    let Some((team_id, citations)) = present_flow::current_award_payload(&state.flow.space_phase)
    else {
        return;
    };
    let team_id = team_id.to_string();
    let citations = citations.to_vec();

    let full_rect = ui.max_rect();
    if let Some(texture) = ensure_award_photo_loaded(state, ctx, &team_id, data_path, config) {
        let tex_size = texture.size_vec2();
        if tex_size.x > 0.0 && tex_size.y > 0.0 {
            let target_aspect = full_rect.width() / full_rect.height().max(1.0);
            let image_aspect = tex_size.x / tex_size.y;
            let uv = if image_aspect > target_aspect {
                let visible_w = target_aspect / image_aspect;
                let u0 = (1.0 - visible_w) * 0.5;
                egui::Rect::from_min_max(egui::pos2(u0, 0.0), egui::pos2(u0 + visible_w, 1.0))
            } else {
                let visible_h = image_aspect / target_aspect.max(0.0001);
                let v0 = (1.0 - visible_h) * 0.5;
                egui::Rect::from_min_max(egui::pos2(0.0, v0), egui::pos2(1.0, v0 + visible_h))
            };
            ui.painter()
                .image(texture.id(), full_rect, uv, egui::Color32::WHITE);
        } else {
            ui.painter()
                .rect_filled(full_rect, 0.0, egui::Color32::from_gray(10));
        }
    } else {
        ui.painter()
            .rect_filled(full_rect, 0.0, egui::Color32::from_gray(10));
    }

    let bar_height = (full_rect.height() * 0.18).clamp(100.0, 220.0);
    let bar_rect = egui::Rect::from_min_max(
        egui::pos2(full_rect.left(), full_rect.bottom() - bar_height),
        egui::pos2(full_rect.right(), full_rect.bottom()),
    );
    ui.painter()
        .rect_filled(bar_rect, 0.0, egui::Color32::from_black_alpha(178));

    let team_name = contest_state
        .teams
        .get(&team_id)
        .map(|team| team.name.clone())
        .unwrap_or_else(|| team_id.clone());
    let award_text = citations.join(" | ");
    let team_font = egui::FontId::proportional((bar_height * 0.3).clamp(28.0, 64.0));
    let award_font = egui::FontId::proportional((bar_height * 0.22).clamp(22.0, 52.0));
    let content_left_pad = bar_rect.width() * 0.03;
    let logo_size = (bar_rect.height() * 0.62).clamp(56.0, 140.0);
    let logo_rect = egui::Rect::from_center_size(
        egui::pos2(
            bar_rect.left() + content_left_pad + logo_size * 0.5,
            bar_rect.center().y,
        ),
        egui::vec2(logo_size, logo_size),
    );
    let text_gap = bar_rect.width() * 0.02;
    let text_left = logo_rect.right() + text_gap;

    if let Some(texture) =
        ensure_logo_loaded(state, ctx, contest_state, &team_id, data_path, config)
    {
        let image = egui::Image::new(&texture)
            .fit_to_exact_size(logo_rect.size())
            .corner_radius(egui::CornerRadius::same((logo_rect.height() * 0.5) as u8));
        ui.put(logo_rect, image);
    } else {
        ui.painter().circle_filled(
            logo_rect.center(),
            logo_rect.height() * 0.5,
            egui::Color32::from_gray(72),
        );
    }

    ui.painter().text(
        egui::pos2(text_left, bar_rect.top() + bar_rect.height() * 0.33),
        egui::Align2::LEFT_CENTER,
        team_name,
        team_font,
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(text_left, bar_rect.top() + bar_rect.height() * 0.73),
        egui::Align2::LEFT_CENTER,
        award_text,
        award_font,
        egui::Color32::WHITE,
    );
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

fn ensure_award_photo_loaded(
    state: &mut PresentUiState,
    ctx: &egui::Context,
    team_id: &str,
    data_path: Option<&str>,
    config: &PyriteConfig,
) -> Option<egui::TextureHandle> {
    if let Some(cached) = state.award_photo_cache.get(team_id) {
        return cached.clone();
    }

    let loaded = state
        .decoded_award_images
        .get(team_id)
        .and_then(|image| {
            image.as_ref().and_then(|img| {
                load_texture_from_decoded(ctx, &format!("team_award_{team_id}"), img)
            })
        })
        .or_else(|| {
            resolve_team_award_photo_path(team_id, data_path, config)
                .and_then(|path| decode_award_image_data(&path, 1920))
                .and_then(|img| {
                    load_texture_from_decoded(ctx, &format!("team_award_{team_id}"), &img)
                })
        })
        .or_else(|| ensure_award_fallback_texture_loaded(state, ctx));
    state
        .award_photo_cache
        .insert(team_id.to_string(), loaded.clone());
    loaded
}

fn ensure_award_fallback_texture_loaded(
    state: &mut PresentUiState,
    ctx: &egui::Context,
) -> Option<egui::TextureHandle> {
    if let Some(cached) = &state.award_fallback_texture {
        return cached.clone();
    }
    let loaded = state
        .decoded_award_fallback
        .as_ref()
        .and_then(|image| image.as_ref())
        .and_then(|img| load_texture_from_decoded(ctx, "team_award_fallback", img));
    state.award_fallback_texture = Some(loaded.clone());
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

fn resolve_award_fallback_path(config: &PyriteConfig) -> Option<PathBuf> {
    let raw_path = config
        .presentation
        .team_photo_fallback_path
        .as_ref()?
        .trim();
    if raw_path.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw_path);
    if path.exists() && path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn resolve_team_award_photo_path(
    team_id: &str,
    data_path: Option<&str>,
    config: &PyriteConfig,
) -> Option<PathBuf> {
    let base = PathBuf::from(data_path?);
    let ext = config
        .presentation
        .team_photo_extension
        .trim()
        .trim_start_matches('.');
    if ext.is_empty() {
        return None;
    }
    let file_path = base.join("teams").join(format!("{team_id}.{ext}"));
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
    load_image_texture(ctx, &format!("team_logo_{team_id}"), path)
}

fn load_image_texture(
    ctx: &egui::Context,
    texture_id: &str,
    path: &Path,
) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(texture_id.to_string(), image, egui::TextureOptions::LINEAR))
}

fn decode_award_image_data(path: &Path, max_dimension: u32) -> Option<DecodedImageData> {
    let bytes = std::fs::read(path).ok()?;
    let mut decoded = image::load_from_memory(&bytes).ok()?;
    let (width, height) = decoded.dimensions();
    let max_side = width.max(height);
    if max_side > max_dimension {
        decoded = decoded.resize(
            max_dimension,
            max_dimension,
            image::imageops::FilterType::Triangle,
        );
    }
    let rgba = decoded.to_rgba8();
    Some(DecodedImageData {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}

fn load_texture_from_decoded(
    ctx: &egui::Context,
    texture_id: &str,
    image: &DecodedImageData,
) -> Option<egui::TextureHandle> {
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.rgba);
    Some(ctx.load_texture(
        texture_id.to_string(),
        color_image,
        egui::TextureOptions::LINEAR,
    ))
}
