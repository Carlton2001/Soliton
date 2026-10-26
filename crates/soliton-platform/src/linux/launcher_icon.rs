//! Palette of the launcher icon installed per user by `install.sh`. Entries and
//! icons installed by any other means are never modified.
use anyhow::{Context, Result};
use soliton_core::config::AppIcon;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Written by `install.sh` in the desktop entry it owns.
pub(super) const MARKER: &str = "X-Soliton-Install=user";

#[derive(Default)]
pub(super) struct LauncherIcon {
    applied: Option<AppIcon>,
}

impl LauncherIcon {
    pub fn apply(&mut self, icon: AppIcon) -> Result<()> {
        if self.applied == Some(icon) {
            return Ok(());
        }
        // Recorded first, so a failure is reported once per palette change.
        self.applied = Some(icon);
        match dirs::data_dir() {
            Some(data) => update(&data, icon),
            None => Ok(()),
        }
    }
}

fn svg(icon: AppIcon) -> &'static [u8] {
    match icon {
        AppIcon::Blue => include_bytes!("../../../../assets/logos/blue.svg"),
        AppIcon::Teal => include_bytes!("../../../../assets/logos/teal.svg"),
        AppIcon::Violet => include_bytes!("../../../../assets/logos/violet.svg"),
        AppIcon::Copper => include_bytes!("../../../../assets/logos/copper.svg"),
        AppIcon::GraphiteMint => include_bytes!("../../../../assets/logos/graphite-mint.svg"),
        AppIcon::IvoryInk => include_bytes!("../../../../assets/logos/ivory-ink.svg"),
    }
}

fn theme(data: &Path) -> PathBuf {
    data.join("icons").join("hicolor")
}

fn icon_file(data: &Path) -> PathBuf {
    theme(data)
        .join("scalable")
        .join("apps")
        .join(format!("{}.svg", crate::APP_ID))
}

fn installed_by_script(data: &Path) -> Result<bool> {
    let entry = data
        .join("applications")
        .join(format!("{}.desktop", crate::APP_ID));
    match std::fs::read_to_string(entry) {
        Ok(content) => Ok(content.lines().any(|line| line.trim() == MARKER)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn update(data: &Path, icon: AppIcon) -> Result<()> {
    if !installed_by_script(data)? {
        return Ok(());
    }
    let target = icon_file(data);
    let content = svg(icon);
    if std::fs::read(&target).is_ok_and(|current| current == content) {
        return Ok(());
    }
    let directory = target.parent().context("Icon has no directory")?;
    std::fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(content)?;
    temporary.as_file().sync_all()?;
    temporary.persist(&target)?;
    // Icon themes look for changes through the theme directory's modification time.
    std::fs::File::open(theme(data))?.set_modified(std::time::SystemTime::now())?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/launcher_icon.rs"]
mod tests;
