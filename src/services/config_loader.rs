use eframe::egui::ahash::HashMap;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct PresentationConfig {
    #[serde(default = "default_rows_per_page")]
    pub rows_per_page: usize,
    #[serde(default = "default_scroll_animation_seconds")]
    pub scroll_animation_seconds: f32,
    #[serde(
        default = "default_row_fly_animation_seconds",
        alias = "row_move_animation_seconds"
    )]
    pub row_fly_animation_seconds: f32,
    #[serde(default = "default_logo_extension")]
    pub logo_extension: String,
}

impl Default for PresentationConfig {
    fn default() -> Self {
        Self {
            rows_per_page: default_rows_per_page(),
            scroll_animation_seconds: default_scroll_animation_seconds(),
            row_fly_animation_seconds: default_row_fly_animation_seconds(),
            logo_extension: default_logo_extension(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PyriteConfig {
    /// This will indicate which submission to filter out.
    /// Will fix the issues like DomJudge using a hard coded non-exisiting team to run problem validation.
    #[serde(default)]
    pub filter_team_submissions: Vec<String>,
    /// This will remap the group of a team to another one.
    /// Will fix issues like a wrong team group that can't be changed before contest finalization.
    #[serde(default)]
    pub team_group_map: HashMap<String, String>,
    #[serde(default)]
    pub presentation: PresentationConfig,
}

fn default_rows_per_page() -> usize {
    12
}

fn default_scroll_animation_seconds() -> f32 {
    0.35
}

fn default_row_fly_animation_seconds() -> f32 {
    0.45
}

fn default_logo_extension() -> String {
    "jpg".to_string()
}

pub fn load_pyrite_config(cdp_folder: &str) -> Result<PyriteConfig, String> {
    let config_path = Path::new(cdp_folder).join("config.toml");
    if !config_path.exists() {
        info!(
            "config.toml not found in CDP folder, using defaults: {}",
            config_path.display()
        );
        return Ok(PyriteConfig::default());
    }

    let raw = fs::read_to_string(&config_path).map_err(|err| {
        format!(
            "Failed to read config.toml at {}: {}",
            config_path.display(),
            err
        )
    })?;

    toml::from_str::<PyriteConfig>(&raw).map_err(|err| {
        format!(
            "Failed to parse config.toml at {}: {}",
            config_path.display(),
            err
        )
    })
}
