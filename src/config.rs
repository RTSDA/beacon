use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub api_url: String,
    pub window_width: i32,
    pub window_height: i32,
    pub slide_interval_seconds: u64,
    pub refresh_interval_minutes: u64,
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;
        let contents = fs::read_to_string(config_path)?;
        let settings: Settings = toml::from_str(&contents)?;
        Ok(settings)
    }

    pub fn slide_interval(&self) -> Duration {
        Duration::from_secs(self.slide_interval_seconds)
    }

    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_interval_minutes * 60)
    }

    fn config_path() -> anyhow::Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("digital-sign");
        path.push("config.toml");
        Ok(path)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_url: String::from("https://api.rockvilletollandsda.church"),
            window_width: 1920,
            window_height: 1080,
            slide_interval_seconds: 10,
            refresh_interval_minutes: 5,
        }
    }
} 