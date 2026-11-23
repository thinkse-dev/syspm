//! Helpers shared by the integration tests.

// Each test file is its own crate and only uses some of these.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The path of a fixture in the repository's `tests/fixtures` directory.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

/// The canonical form of `path`, which is what `find_project_root` returns. It can differ from
/// `path` itself. For example, macOS temporary directories are reached through a symbolic link.
pub fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

/// Creates a file, along with any missing parent directories.
pub fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
