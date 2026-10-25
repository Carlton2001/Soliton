//! Per-user registration, touched only after an explicit preference change.
use anyhow::{Context, Result};
use std::path::Path;
use windows_registry::{CURRENT_USER, Key};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "Soliton";

fn command(executable: &Path) -> Result<String> {
    let path = executable
        .to_str()
        .context("Executable path is not Unicode")?;
    anyhow::ensure!(!path.contains('"'), "Executable path contains a quote");
    // The quotes are required for paths containing spaces; never invoke a shell.
    let command = format!("\"{path}\"");
    anyhow::ensure!(
        command.encode_utf16().count() <= 260,
        "Autostart command exceeds the Windows Run limit"
    );
    Ok(command)
}

pub(super) fn enabled() -> Result<bool> {
    let key = match CURRENT_USER.open(RUN) {
        Ok(key) => key,
        Err(error) if error.code().0 == 0x80070002u32 as i32 => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    registered(&key, &command(&std::env::current_exe()?)?)
}

fn registered(key: &Key, expected: &str) -> Result<bool> {
    match key.get_string(VALUE) {
        Ok(value) => Ok(value == expected),
        Err(error) if error.code().0 == 0x80070002u32 as i32 => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn set(enabled: bool) -> Result<()> {
    let key = CURRENT_USER.create(RUN)?;
    configure(
        &key,
        enabled
            .then(|| command(&std::env::current_exe()?))
            .transpose()?,
    )
}

fn configure(key: &Key, command: Option<String>) -> Result<()> {
    if let Some(command) = command {
        key.set_string(VALUE, command)?;
    } else {
        match key.remove_value(VALUE) {
            Ok(()) => {}
            Err(error) if error.code().0 == 0x80070002u32 as i32 => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/autostart.rs"]
mod tests;
