//! Folders chosen through the file chooser portal arrive under the document
//! portal, where file notifications are not delivered and a removed disk still
//! looks reachable. When the sandbox can read the folder itself, its host path
//! is used instead.
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

/// Mount points of the document portal: inside a Flatpak, then on the host.
fn mount_points() -> Vec<PathBuf> {
    let mut mounts = vec![PathBuf::from("/run/flatpak/doc")];
    mounts.extend(std::env::var_os("XDG_RUNTIME_DIR").map(|run| PathBuf::from(run).join("doc")));
    mounts
}

/// The document identifier and the path below the document itself, for a path
/// of the portal. A document appears as `<mount>/<id>/<its own name>`, and
/// `GetHostPaths` returns the host path of that named entry.
fn split<'a>(path: &'a Path, mount: &Path) -> Option<(String, &'a Path)> {
    let mut rest = path.strip_prefix(mount).ok()?.components();
    let (Some(Component::Normal(id)), Some(Component::Normal(_))) = (rest.next(), rest.next())
    else {
        return None;
    };
    Some((id.to_str()?.to_owned(), rest.as_path()))
}

/// Blocks on the session bus: call from a worker. Any failure keeps `path`.
pub fn host_path(path: PathBuf) -> PathBuf {
    let Some((id, rest)) = mount_points().iter().find_map(|mount| split(&path, mount)) else {
        return path;
    };
    let Ok(host) = document_host_path(&id) else {
        return path;
    };
    let host = if rest.as_os_str().is_empty() {
        host
    } else {
        host.join(rest)
    };
    if std::fs::read_dir(&host).is_ok() {
        host
    } else {
        path
    }
}

fn document_host_path(id: &str) -> zbus::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let connection = zbus::blocking::Connection::session()?;
    let reply = connection.call_method(
        Some("org.freedesktop.portal.Documents"),
        "/org/freedesktop/portal/documents",
        Some("org.freedesktop.portal.Documents"),
        "GetHostPaths",
        &(vec![id],),
    )?;
    let mut paths: HashMap<String, Vec<u8>> = reply.body().deserialize()?;
    let mut bytes = paths
        .remove(id)
        .ok_or_else(|| zbus::Error::Failure("Unknown document".into()))?;
    // Byte strings of the portal end with a NUL.
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    Ok(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

#[cfg(test)]
#[path = "tests/documents.rs"]
mod tests;
