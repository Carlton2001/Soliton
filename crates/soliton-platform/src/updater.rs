//! Signed installer staging, adapted from Oxyde-GPUI (Carlton2001, GPL-3.0).
use base64::Engine;
use soliton_core::updates::{Release, Target};
use std::{borrow::Cow, io, path::PathBuf};

const ASSET_LIMIT: usize = 512 * 1024 * 1024;

/// Detect the installation on a worker before checking for updates.
pub fn target() -> Option<Target> {
    #[cfg(target_os = "windows")]
    {
        Some(Target::Windows)
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        crate::linux::updates::installed_executable().map(|_| Target::LinuxArchive)
    }
    #[cfg(not(any(
        target_os = "windows",
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    None
}

pub fn signing_key() -> Option<Cow<'static, str>> {
    if let Some(key) = option_env!("SOLITON_UPDATE_PUBLIC_KEY").filter(|key| !key.trim().is_empty())
    {
        return Some(Cow::Borrowed(key));
    }
    #[cfg(target_os = "linux")]
    {
        Some(Cow::Owned(
            base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                "../../../installer/signing/soliton-update.pub"
            )),
        ))
    }
    #[cfg(not(target_os = "linux"))]
    None
}

pub struct PreparedUpdate {
    directory: tempfile::TempDir,
    setup: PathBuf,
}

/// A portable ZIP is updated manually: never install a second copy silently.
pub fn is_portable() -> io::Result<bool> {
    if !cfg!(target_os = "windows") {
        return Ok(false);
    }
    let exe = std::env::current_exe()?;
    Ok(exe
        .parent()
        .is_some_and(|parent| parent.join("data").is_dir()))
}

/// Call on a worker. No file is written before the signature is verified.
pub fn prepare(
    release: &Release,
    public_key: &str,
    target: Target,
) -> Result<PreparedUpdate, String> {
    if public_key.trim().is_empty() {
        return Err("Missing Soliton update signing key".into());
    }
    let bytes = crate::download_https(&release.setup, ASSET_LIMIT).map_err(|e| e.to_string())?;
    stage(
        &bytes,
        &release.signature,
        public_key,
        &release.version,
        target,
    )
}

fn stage(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
    version: &str,
    target: Target,
) -> Result<PreparedUpdate, String> {
    verify(bytes, signature, public_key, version)?;
    let directory = tempfile::Builder::new()
        .prefix("soliton-update-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let setup = directory.path().join(target.filename(version));
    std::fs::write(&setup, bytes).map_err(|e| e.to_string())?;
    Ok(PreparedUpdate { directory, setup })
}

/// The trusted comment, covered by the signature, names the version, so that an
/// older signed installer cannot be offered again as a newer release.
fn verify(bytes: &[u8], signature: &str, public_key: &str, version: &str) -> Result<(), String> {
    let text = |encoded: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    };
    let key = minisign_verify::PublicKey::decode(&text(public_key)?).map_err(|e| e.to_string())?;
    let signature =
        minisign_verify::Signature::decode(&text(signature)?).map_err(|e| e.to_string())?;
    key.verify(bytes, &signature, false)
        .map_err(|e| e.to_string())?;
    let expected = format!("soliton-version:{version}");
    if !signature
        .trusted_comment()
        .split(['\t', ' '])
        .any(|field| field == expected)
    {
        return Err(format!("Installer signature is not for version {version}"));
    }
    Ok(())
}

impl PreparedUpdate {
    /// Starts Setup, which waits for Soliton's PID before replacing files.
    pub fn launch(self) -> io::Result<()> {
        #[cfg(target_os = "windows")]
        {
            use std::{
                os::windows::process::CommandExt,
                process::{Command, Stdio},
            };
            let exe = std::env::current_exe()?;
            let directory = exe
                .parent()
                .ok_or_else(|| io::Error::other("Missing executable directory"))?;
            Command::new(&self.setup)
                .args(["/UPDATE", "/SILENT", "/NORESTART"])
                .arg(format!("/WAITPID={}", std::process::id()))
                .arg(format!("/DIR={}", directory.display()))
                .creation_flags(0x08000000)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            // Setup needs this file after Soliton exits. Failed launches clean up via Drop.
            let _ = self.directory.keep();
            Ok(())
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            crate::linux::updates::install(&self.setup, self.directory.path())
        }
        #[cfg(not(any(
            target_os = "windows",
            all(target_os = "linux", target_arch = "x86_64")
        )))]
        {
            let _ = (&self.setup, &self.directory);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Installer updates are unavailable on this platform",
            ))
        }
    }
}

#[cfg(test)]
#[path = "tests/updater.rs"]
mod tests;
