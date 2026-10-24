use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

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

/// Application icon palette, independent of the interface accent and theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppIcon {
    Blue,
    Teal,
    Violet,
    Copper,
    #[default]
    GraphiteMint,
    IvoryInk,
}

impl AppIcon {
    pub const ALL: [Self; 6] = [
        Self::Blue,
        Self::Teal,
        Self::Violet,
        Self::Copper,
        Self::GraphiteMint,
        Self::IvoryInk,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Teal => "teal",
            Self::Violet => "violet",
            Self::Copper => "copper",
            Self::GraphiteMint => "graphite-mint",
            Self::IvoryInk => "ivory-ink",
        }
    }
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

/// Main page of the interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Home,
    Artists,
    /// Former Genres and Years pages reopen on the albums.
    #[default]
    #[serde(alias = "Genres", alias = "Years")]
    Albums,
    Playlists,
    Queue,
    Settings,
}

/// Where the user was when the application closed, restored at startup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LastView {
    pub playlist: Option<i64>,
    pub view: View,
    /// Open album sheet, when it still exists in the library.
    pub album: Option<i64>,
    /// Artist whose albums were listed.
    pub artist: Option<String>,
}

/// Play queue reopened paused at startup. Library track identifiers only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LastQueue {
    /// Parallel to tracks; absent in configurations saved before playlist playback.
    pub playlist_entries: Vec<Option<i64>>,
    pub tracks: Vec<i64>,
    pub current: Option<i64>,
    /// Occurrence index distinguishes repeated tracks in a manual queue.
    pub current_index: Option<usize>,
    pub position_ms: u64,
}

/// Window size and position in logical pixels; the restored size when maximized.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowPlacement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Off,
    Track,
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloseBehavior {
    #[default]
    Quit,
    Hide,
    Ask,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub appearance: Appearance,
    pub accent: Accent,
    pub app_icon: AppIcon,
    pub album_grid: bool,
    pub artist_grid: bool,
    pub cover_size: CoverSize,
    pub album_sort: AlbumSort,
    pub hide_unavailable: bool,
    pub volume: f32,
    /// Local CPAL device name; None uses the system default.
    pub output_device: Option<String>,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub close_behavior: CloseBehavior,
    /// Pixels per rem; every interface dimension scales with it.
    pub ui_rem_size: u32,
    pub last_view: LastView,
    pub last_queue: LastQueue,
    pub window: Option<WindowPlacement>,
}

impl Settings {
    pub const DEFAULT_REM_SIZE: u32 = 16;
    pub const MIN_REM_SIZE: u32 = 12;
    pub const MAX_REM_SIZE: u32 = 24;

    pub fn clamp_rem_size(value: u32) -> u32 {
        value.clamp(Self::MIN_REM_SIZE, Self::MAX_REM_SIZE)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::French,
            appearance: Appearance::System,
            accent: Accent::Teal,
            app_icon: AppIcon::default(),
            album_grid: true,
            artist_grid: true,
            cover_size: CoverSize::Medium,
            album_sort: AlbumSort::Artist,
            hide_unavailable: false,
            volume: 0.7,
            output_device: None,
            shuffle: false,
            repeat: Repeat::Off,
            close_behavior: CloseBehavior::Quit,
            ui_rem_size: Self::DEFAULT_REM_SIZE,
            last_view: LastView::default(),
            last_queue: LastQueue::default(),
            window: None,
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
                settings.ui_rem_size = Self::clamp_rem_size(settings.ui_rem_size);
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
        let mut file = fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        // On disk before it replaces the previous document: a power loss leaves
        // one complete version rather than an empty file.
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, path)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
