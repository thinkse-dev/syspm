//! Finds a project's root directory and its model interchange files.
//!
//! This is syspm's own behaviour, since the standard doesn't define how a working directory maps
//! to a project. It depends only on the file system, not on any version control system. It deals
//! with single projects, and a project inside a larger project is a separate project.
//!
//! The rules are in docs/specs/project-metadata.md, section 3.

use crate::interchange::PROJECT_MANIFEST;
use std::convert::Infallible;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A project couldn't be found, or its files couldn't be listed.
///
/// The message names the path. Any underlying cause is in [`std::error::Error::source`], not
/// repeated in the message.
#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("no project found at or above {}", start.display())]
    ProjectNotFound { start: PathBuf },
    #[error("cannot read {}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("model file name is not valid UTF-8: {}", path.display())]
    NonUtf8Path { path: PathBuf },
}

/// The result of [`scan_model_files`].
///
/// `files` is incomplete whenever `failures` isn't empty, and there's no telling what a failure
/// hid. A directory that couldn't be read may hold any number of model files.
#[derive(Debug)]
pub struct ModelFileScan {
    /// The model files that could be reached, in the form [`list_model_files`] gives.
    pub files: Vec<String>,
    /// Everything that couldn't be listed.
    pub failures: Vec<DiscoverError>,
}

impl ModelFileScan {
    /// Whether discovery reached the whole project, so that `files` lists every file the
    /// discovery policy of [`list_model_files`] covers.
    ///
    /// It doesn't mean every model interchange file was found. Files in other formats, such as
    /// JSON or XMI, are outside that policy.
    pub fn is_complete(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Finds the nearest directory at or above `start` that contains `.project.json`.
///
/// The search stops only at the file system root. `start` can be relative, and can be a file.
/// The result is in canonical form: absolute, with symbolic links resolved.
///
/// Anything named `.project.json` other than a directory ends the search, even if it can't be
/// read as a manifest. [`crate::read_manifest`] then reports the problem. So that this also holds
/// when `start` is the manifest itself, a symbolic link there isn't resolved.
pub fn find_project_root(start: &Path) -> Result<PathBuf, DiscoverError> {
    let start = search_start(start).map_err(|source| DiscoverError::Io {
        path: start.to_path_buf(),
        source,
    })?;
    match start.ancestors().find(|dir| is_project_root(dir)) {
        Some(root) => Ok(root.to_path_buf()),
        None => Err(DiscoverError::ProjectNotFound { start }),
    }
}

/// The canonical directory or file to search upward from.
///
/// A manifest starts the search from its parent. Resolving a link at `.project.json` would start
/// in the project it points to, and the link itself would never reach [`crate::read_manifest`].
fn search_start(start: &Path) -> io::Result<PathBuf> {
    if start
        .file_name()
        .is_some_and(|name| name == PROJECT_MANIFEST)
        && !fs::symlink_metadata(start)?.is_dir()
    {
        fs::canonicalize(
            start
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )
    } else {
        fs::canonicalize(start)
    }
}

/// Lists the model files that syspm discovers in the project in `root`, sorted, as paths relative
/// to `root` with `/` separators.
///
/// This is syspm's discovery policy, not an inventory of every model interchange file. It finds
/// only the textual notations, `.sysml` and `.kerml`. KerML 1.0 section 10.2 allows other
/// interchange formats, but `.json` in particular would match unrelated files. Such files can
/// still be named explicitly in `.meta.json`. Hidden directories, symbolic links, and
/// subdirectories that are projects themselves are skipped.
pub fn list_model_files(root: &Path) -> Result<Vec<String>, DiscoverError> {
    let mut files = Vec::new();
    walk_model_files(root, Path::new(""), &mut files, &mut Err)?;
    files.sort();
    Ok(files)
}

/// Lists what [`list_model_files`] would, but carries on past failures. Returns the files it
/// could reach, along with every failure.
///
/// Use this when a partial list is still useful. Before treating the list as the whole project,
/// check [`ModelFileScan::is_complete`].
pub fn scan_model_files(root: &Path) -> ModelFileScan {
    let mut files = Vec::new();
    let mut failures = Vec::new();
    let mut record = |failure| {
        failures.push(failure);
        Ok::<(), Infallible>(())
    };
    let Ok(()) = walk_model_files(root, Path::new(""), &mut files, &mut record);
    files.sort();
    ModelFileScan { files, failures }
}

/// Adds the model files in `relative`, a directory relative to `root`, to `files`.
///
/// Each failure goes to `on_failure`, which decides what happens next. Returning `Err` stops the
/// walk, and returning `Ok` skips what failed and carries on with the rest.
fn walk_model_files<E>(
    root: &Path,
    relative: &Path,
    files: &mut Vec<String>,
    on_failure: &mut impl FnMut(DiscoverError) -> Result<(), E>,
) -> Result<(), E> {
    let dir = root.join(relative);
    let read_error = |source| DiscoverError::Io {
        path: dir.clone(),
        source,
    };

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(source) => return on_failure(read_error(source)),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                // The rest of this directory can't be read, so move on from it.
                on_failure(read_error(source))?;
                break;
            }
        };
        // `DirEntry::file_type` describes a symbolic link as a link, not as what it points to,
        // so links are neither directories nor files here.
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(source) => {
                on_failure(read_error(source))?;
                continue;
            }
        };
        let path = relative.join(entry.file_name());

        if file_type.is_dir() {
            let hidden = entry.file_name().as_encoded_bytes().starts_with(b".");
            if !hidden && !is_project_root(&entry.path()) {
                walk_model_files(root, &path, files, on_failure)?;
            }
        } else if file_type.is_file() && has_textual_model_extension(&path) {
            match to_slash_path(&path) {
                Some(slash_path) => files.push(slash_path),
                None => on_failure(DiscoverError::NonUtf8Path {
                    path: root.join(&path),
                })?,
            }
        }
    }
    Ok(())
}

/// Whether `dir` contains anything but a directory named `.project.json`.
///
/// A manifest that is a symbolic link or named pipe is still an attempt at a project, so this
/// doesn't require a regular file. Otherwise discovery would silently move on to a parent project.
pub(crate) fn is_project_root(dir: &Path) -> bool {
    fs::symlink_metadata(dir.join(PROJECT_MANIFEST)).is_ok_and(|metadata| !metadata.is_dir())
}

/// Whether `path` has the extension of a textual notation file, which is all that discovery
/// looks for. See [`list_model_files`].
fn has_textual_model_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "sysml" || extension == "kerml")
}

/// Joins the components of a relative path with `/`, the separator KerML uses on every platform
/// (Table 13 note 2). Returns `None` if a component isn't valid UTF-8, because metadata records
/// paths as JSON strings and such a path couldn't be recorded.
pub(crate) fn to_slash_path(path: &Path) -> Option<String> {
    let components: Option<Vec<&str>> = path.iter().map(|part| part.to_str()).collect();
    Some(components?.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_path_components_with_slashes() {
        let path: PathBuf = ["models", "deep", "Deep.kerml"].iter().collect();
        assert_eq!(
            to_slash_path(&path).as_deref(),
            Some("models/deep/Deep.kerml")
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_has_no_slash_form() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(OsStr::from_bytes(b"models/\xFF.sysml"));
        assert_eq!(to_slash_path(path), None);
    }
}
