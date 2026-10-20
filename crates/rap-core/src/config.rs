use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    French,
    English,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub dark: bool,
    pub album_grid: bool,
    pub volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::French,
            dark: true,
            album_grid: true,
            volume: 0.7,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(bytes) => {
                let mut settings: Self = serde_json::from_slice(&bytes)?;
                settings.volume = settings.volume.clamp(0.0, 1.0);
                Ok(settings)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    /// Call only from a service worker; preserve the last complete document on failure.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
