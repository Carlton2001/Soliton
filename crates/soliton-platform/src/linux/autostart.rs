//! XDG autostart entry, touched only after an explicit preference change.
use anyhow::{Context, Result};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

fn file(config: &Path) -> PathBuf {
    config
        .join("autostart")
        .join(format!("{}.desktop", crate::APP_ID))
}

fn location() -> Result<PathBuf> {
    Ok(file(
        &dirs::config_dir().context("No XDG configuration directory")?,
    ))
}

/// Quotes one `Exec` argument as the Desktop Entry specification requires;
/// the session starts the executable directly, never through a shell.
fn exec(executable: &Path) -> Result<String> {
    let path = executable
        .to_str()
        .context("Executable path is not Unicode")?;
    anyhow::ensure!(
        !path.chars().any(char::is_control),
        "Executable path contains a control character"
    );
    let mut argument = String::from('"');
    for character in path.chars() {
        match character {
            '"' | '`' | '$' | '\\' => {
                argument.push('\\');
                argument.push(character);
            }
            '%' => argument.push_str("%%"),
            _ => argument.push(character),
        }
    }
    argument.push('"');
    // String values escape backslashes once more.
    Ok(argument.replace('\\', "\\\\"))
}

fn entry(executable: &Path) -> Result<String> {
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=Soliton\nExec={}\nIcon={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec(executable)?,
        crate::APP_ID
    ))
}

pub(super) fn enabled() -> Result<bool> {
    registered(&location()?, &entry(&std::env::current_exe()?)?)
}

fn registered(file: &Path, expected: &str) -> Result<bool> {
    match std::fs::read_to_string(file) {
        Ok(content) => Ok(content == expected),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn set(enabled: bool) -> Result<()> {
    configure(
        &location()?,
        enabled
            .then(|| entry(&std::env::current_exe()?))
            .transpose()?,
    )
}

fn configure(file: &Path, entry: Option<String>) -> Result<()> {
    let Some(entry) = entry else {
        return match std::fs::remove_file(file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        };
    };
    let directory = file.parent().context("Autostart entry has no directory")?;
    std::fs::create_dir_all(directory)?;
    // A session starting meanwhile reads either the old or the new entry.
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(entry.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(file)?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/autostart.rs"]
mod tests;
