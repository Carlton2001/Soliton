//! Platform boundaries. Native APIs must stay behind target-specific modules.
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub database: PathBuf,
    pub covers: PathBuf,
    pub settings: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
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
