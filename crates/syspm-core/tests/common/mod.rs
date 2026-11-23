//! Helpers shared by the integration tests.

// Each test file is its own crate and only uses some of these.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The path of a fixture in the repository's `tests/fixtures` directory.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}
