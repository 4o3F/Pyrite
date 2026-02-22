use eframe::egui::ahash::HashMap;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::info;

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
