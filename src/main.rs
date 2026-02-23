mod models;
mod screens;
mod services;

use eframe::egui;
use screens::load_data::LoadDataAction;
use screens::present::PresentAction;
use screens::set_award::SetAwardAction;
use services::config_loader::PyriteConfig;
use std::fs;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

enum PyriteState {
    LoadData,
    SetAward,
    Present,
}

struct PyriteApp {
    state: PyriteState,
    data_path: Option<String>,
    contest_state: Option<models::ContestState>,
    config: PyriteConfig,
}

impl Default for PyriteApp {
    fn default() -> Self {
        Self {
            state: PyriteState::LoadData,
            data_path: None,
            contest_state: None,
            config: PyriteConfig::default(),
        }
    }
}

impl eframe::App for PyriteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            match self.state {
                PyriteState::LoadData => {
                    ui.vertical_centered(|ui| {
                        if let LoadDataAction::Continue = screens::load_data::ui(ui, &mut self.data_path)
                        {
                            if let Some(parsed_state) =
                                screens::load_data::take_parsed_contest_state()
                            {
                                if let Some(config) = screens::load_data::take_parsed_config() {
                                    self.config = config;
                                    self.contest_state = Some(parsed_state);
                                    info!("Transition: LoadData -> SetAward");
                                    self.state = PyriteState::SetAward;
                                } else {
                                    warn!("Cannot continue: parsed config is missing");
                                }
                            } else {
                                info!("Cannot continue: parsed contest state is missing");
                            }
                        }
                    });
                }
                PyriteState::SetAward => {
                    if let Some(contest_state) = self.contest_state.as_mut() {
                        match screens::set_award::ui(ui, contest_state) {
                            SetAwardAction::Continue => {
                                info!("Transition: SetAward -> Present");
                                self.state = PyriteState::Present;
                            }
                            SetAwardAction::Stay => {}
                        }
                    } else {
                        ui.colored_label(
                            egui::Color32::RED,
                            "Contest data missing. Go back to Load Data.",
                        );
                    }
                }
                PyriteState::Present => {
                    if let Some(contest_state) = self.contest_state.as_mut() {
                        match screens::present::ui(
                            ui,
                            ctx,
                            contest_state,
                            self.data_path.as_deref(),
                            &self.config,
                        ) {
                            PresentAction::Stay => {}
                        }
                    } else {
                        ui.colored_label(
                            egui::Color32::RED,
                            "Contest data missing. Go back to Load Data.",
                        );
                    }
                }
            }
        });
    }
}

fn init_tracing() -> Option<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true);

    let _ = fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::daily("logs", "pyrite.log");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file_writer)
        .with_target(true);

    let init_result = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init();

    if let Err(err) = init_result {
        eprintln!("tracing init failed: {err}");
        return None;
    }

    Some(file_guard)
}

fn install_embedded_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto_sans_cjk".to_string(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansCJKsc-Regular.otf"
        ))),
    );

    if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        proportional.insert(0, "noto_sans_cjk".to_string());
    }
    if let Some(monospace) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        monospace.insert(0, "noto_sans_cjk".to_string());
    }

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let _log_guard = init_tracing();
    info!("Starting Pyrite");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Pyrite",
        options,
        Box::new(|cc| {
            install_embedded_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_pixels_per_point(1.1);

            let mut style = (*cc.egui_ctx.style()).clone();
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(34.0));
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(22.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(22.0));
            style.spacing.button_padding = egui::vec2(14.0, 9.0);
            cc.egui_ctx.set_style(style);

            Ok(Box::new(PyriteApp::default()))
        }),
    )
}
