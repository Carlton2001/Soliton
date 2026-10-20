use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    French,
    English,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    /// Follows the light or dark mode of the operating system.
    #[default]
    System,
    Light,
    Dark,
}

/// Predefined accent hues; each theme supplies a readable variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Teal,
    Blue,
    Indigo,
    Violet,
    Pink,
    Red,
    Orange,
    Green,
}

impl Accent {
    pub const ALL: [Accent; 8] = [
        Accent::Teal,
        Accent::Blue,
        Accent::Indigo,
        Accent::Violet,
        Accent::Pink,
        Accent::Red,
        Accent::Orange,
        Accent::Green,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlbumSort {
    #[default]
    Artist,
    Title,
    Year,
    /// Most recently added first.
    Added,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub appearance: Appearance,
    pub accent: Accent,
    pub album_grid: bool,
    pub cover_size: CoverSize,
    pub album_sort: AlbumSort,
    pub hide_unavailable: bool,
    pub volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::French,
            appearance: Appearance::System,
            accent: Accent::Teal,
            album_grid: true,
            cover_size: CoverSize::Medium,
            album_sort: AlbumSort::Artist,
            hide_unavailable: false,
            volume: 0.7,
        }
    }
}

/// Fields of earlier versions, read once to preserve the user's choices.
#[derive(Deserialize)]
struct Legacy {
    dark: Option<bool>,
    appearance: Option<Appearance>,
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(bytes) => {
                let mut settings: Self = serde_json::from_slice(&bytes)?;
                let legacy: Legacy = serde_json::from_slice(&bytes)?;
                if let (None, Some(dark)) = (legacy.appearance, legacy.dark) {
                    settings.appearance = if dark {
                        Appearance::Dark
                    } else {
                        Appearance::Light
                    };
                }
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
