//! Archive installation and relaunch adapted from Oxyde-GPUI (Carlton2001, GPL-3.0).
use std::os::unix::process::CommandExt;
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Match the install.sh launcher and executable, including installations on
/// systems where the home directory itself is reached through a symlink.
fn archive_executable(executable: &Path, home: &Path, data: &Path) -> Option<PathBuf> {
    let path = home.join(".local/bin/soliton");
    (std::fs::symlink_metadata(&path).ok()?.is_file()
        && path.canonicalize().ok()? == executable
        && super::launcher_icon::installed_by_script(data).ok()?)
    .then_some(path)
}

pub(crate) fn installed_executable() -> Option<PathBuf> {
    archive_executable(
        &std::env::current_exe().ok()?,
        &dirs::home_dir()?,
        &dirs::data_dir()?,
    )
}

pub(crate) fn download(url: &str, limit: usize) -> io::Result<Vec<u8>> {
    let mut child = Command::new("curl")
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-redirs",
            "5",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "15",
            "--max-time",
            "180",
            "--max-filesize",
        ])
        .arg(limit.to_string())
        .arg("--url")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .expect("curl stdout was piped")
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > limit {
        let _ = child.kill();
    }
    let status = child.wait()?;
    read?;
    if bytes.len() > limit {
        return Err(io::Error::other("Update download exceeds its size limit"));
    }
    if !status.success() {
        return Err(io::Error::other(format!(
            "Update download failed ({status})"
        )));
    }
    Ok(bytes)
}

pub(crate) fn install(archive: &Path, directory: &Path) -> io::Result<()> {
    let executable = installed_executable()
        .ok_or_else(|| io::Error::other("Soliton was not installed by install.sh"))?;
    install_archive(archive, directory, std::process::id())?;
    relaunch_after(std::process::id(), &executable)
}

fn install_archive(archive: &Path, directory: &Path, process: u32) -> io::Result<()> {
    let root = archive
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".tar.gz"))
        .ok_or_else(|| io::Error::other("Invalid update archive filename"))?;
    // The caller has verified the complete archive before staging it privately.
    let output = Command::new("tar")
        .env_remove("TAR_OPTIONS")
        .env_remove("GZIP")
        .args([
            "--extract",
            "--gzip",
            "--no-same-owner",
            "--no-same-permissions",
            "--file",
        ])
        .arg(archive)
        .arg("--directory")
        .arg(directory)
        .arg("--")
        .arg(root)
        .output()?;
    if !output.status.success() {
        return Err(command_error("Extracting the update", &output));
    }
    let output = Command::new("sh")
        .arg(directory.join(root).join("install.sh"))
        .arg("--update")
        .arg(process.to_string())
        .output()?;
    if !output.status.success() {
        return Err(command_error("Installing the update", &output));
    }
    Ok(())
}

fn command_error(action: &str, output: &std::process::Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    io::Error::other(format!(
        "{action} failed ({}): {}",
        output.status,
        stderr.trim()
    ))
}

fn relaunch_after(process: u32, executable: &Path) -> io::Result<()> {
    // Wait for normal shutdown to flush the playback session and settings.
    Command::new("sh")
        .arg("-c")
        .arg(r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec "$2""#)
        .arg("soliton-relaunch")
        .arg(process.to_string())
        .arg(executable)
        // GPUI consumes a launcher-supplied Wayland socket on first startup.
        .env_remove("WAYLAND_SOCKET")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(drop)
}

#[cfg(test)]
#[path = "tests/updates.rs"]
mod tests;
