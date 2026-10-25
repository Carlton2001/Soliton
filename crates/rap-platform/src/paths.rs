//! Installed and portable application storage, independent of the UI.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub database: PathBuf,
    pub covers: PathBuf,
    pub settings: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let executable = std::env::current_exe()?;
        if let Some(portable) = portable_directory(&executable) {
            return Ok(Self::at(portable));
        }
        let root = dirs::data_local_dir()
            .context("No local application data directory")?
            .join("RAP");
        Ok(Self::at(root))
    }

    pub fn at(root: PathBuf) -> Self {
        Self {
            database: root.join("library.sqlite3"),
            covers: root.join("covers"),
            settings: root.join("settings.json"),
            root,
        }
    }

    pub fn create(&self) -> Result<()> {
        std::fs::create_dir_all(&self.covers)?;
        Ok(())
    }
}

fn portable_directory(executable: &Path) -> Option<PathBuf> {
    let data = executable.parent()?.join("data");
    data.is_dir().then_some(data)
}

#[cfg(test)]
#[path = "tests/paths.rs"]
mod tests;
