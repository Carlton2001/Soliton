//! Release manifest policy, adapted from Oxyde-GPUI (Carlton2001, GPL-3.0).
//! No UI, network requests or installer execution.
pub const LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub setup: String,
    pub signature: String,
}

#[derive(serde::Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    platforms: std::collections::HashMap<String, Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    url: String,
    signature: String,
}

pub fn valid_repository(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
}

fn version(value: &str) -> Result<[u64; 3], String> {
    let parts = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return Err("Invalid version".to_owned());
            }
            part.parse::<u64>()
                .map_err(|_| "Invalid version".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    parts
        .try_into()
        .map_err(|_| "Expected major.minor.patch".to_owned())
}

pub fn parse(bytes: &[u8], current: &str, repository: &str) -> Result<Option<Release>, String> {
    if bytes.len() > LIMIT || !valid_repository(repository) {
        return Err("Invalid update manifest".into());
    }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let candidate = version(&manifest.version)?;
    if candidate <= version(current)? {
        return Ok(None);
    }
    let version = candidate.map(|part| part.to_string()).join(".");
    let asset = manifest
        .platforms
        .get("windows-x86_64")
        .ok_or("Missing Windows installer")?;
    let expected = format!(
        "https://github.com/{repository}/releases/download/v{version}/RAP_{version}_x64-setup.exe"
    );
    if asset.url != expected || asset.signature.trim().is_empty() {
        return Err("Invalid installer location or missing signature".into());
    }
    Ok(Some(Release {
        page: format!("https://github.com/{repository}/releases/tag/v{version}"),
        version,
        setup: asset.url.clone(),
        signature: asset.signature.clone(),
    }))
}

#[cfg(test)]
#[path = "tests/updates.rs"]
mod tests;
