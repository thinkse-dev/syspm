//! Creates the manifest of a new project.

use crate::interchange::PROJECT_MANIFEST;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A project manifest couldn't be created.
///
/// The message names the path, if there is one. Any underlying cause is in
/// [`std::error::Error::source`], not repeated in the message.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("project manifest already exists: {}", path.display())]
    AlreadyExists { path: PathBuf },
    #[error("cannot write project manifest {}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("cannot determine a project name from {}", dir.display())]
    NoDefaultName { dir: PathBuf },
    #[error("project name is empty")]
    EmptyName,
}

/// Creates `.project.json` in `dir` and returns its path.
///
/// `name` defaults to the name of `dir`, resolved so that a path such as `.` has one. An empty
/// name is an error. A project above `dir` isn't an error, since the new project is a separate
/// project.
///
/// Anything already named `.project.json`, even a broken symbolic link, is left alone. The file
/// is created in one step, so nothing can appear between a check and the write. If the write
/// fails partway, the partial file is removed so that the next attempt can start over.
pub fn init_project(dir: &Path, name: Option<&str>, version: &str) -> Result<PathBuf, InitError> {
    let path = dir.join(PROJECT_MANIFEST);
    let name = match name {
        Some(name) => name.to_owned(),
        None => match default_name(dir) {
            Ok(Some(name)) => name,
            Ok(None) => {
                return Err(InitError::NoDefaultName {
                    dir: dir.to_path_buf(),
                });
            }
            Err(source) => return Err(InitError::Io { path, source }),
        },
    };
    if name.is_empty() {
        return Err(InitError::EmptyName);
    }

    match write_manifest(&path, &name, version) {
        Ok(()) => Ok(path),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            Err(InitError::AlreadyExists { path })
        }
        Err(source) => Err(InitError::Io { path, source }),
    }
}

/// The name of `dir`, or `None` if it has no name (the file system root) or the name isn't valid
/// UTF-8, which a JSON string couldn't hold faithfully.
fn default_name(dir: &Path) -> io::Result<Option<String>> {
    let canonical = fs::canonicalize(dir)?;
    Ok(canonical
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned))
}

fn write_manifest(path: &Path, name: &str, version: &str) -> io::Result<()> {
    // Serialize first, so that a failure here leaves no empty file behind.
    let mut json = manifest_json(name, version)?;
    json.push('\n');
    let mut file = File::create_new(path)?;
    if let Err(error) = file.write_all(json.as_bytes()) {
        // This call just created the file, so removing it can't lose anyone else's data. Close it
        // first, because Windows can't remove an open file. If removal fails too, the write error
        // is the one to report.
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

/// The properties a new manifest has.
///
/// This is separate from [`crate::ProjectManifest`] because `#[serde(remote = "Self")]` on that
/// type would stop a derived `Serialize` from implementing the trait.
#[derive(Serialize)]
struct NewManifest<'a> {
    name: &'a str,
    version: &'a str,
}

fn manifest_json(name: &str, version: &str) -> serde_json::Result<String> {
    let manifest = NewManifest { name, version };
    serde_json::to_string_pretty(&manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProjectManifest, validate_manifest};

    #[test]
    fn generates_valid_manifest_json() {
        for name in ["my-project", "my-\"project\""] {
            let json = manifest_json(name, "0.1.0").unwrap();
            let manifest: ProjectManifest = serde_json::from_str(&json).unwrap();
            assert_eq!(manifest.name, name);
            assert_eq!(manifest.version, "0.1.0");
            assert!(validate_manifest(&manifest).is_empty(), "{name}");
        }
    }
}
