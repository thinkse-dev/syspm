//! The library behind syspm. All of the package-management logic lives here, so any front end
//! can use it.
//!
//! So far it reads a project's `.project.json`. Validation, `.meta.json` and everything to do with
//! dependencies come later.

mod interchange;

pub use interchange::{ProjectManifest, ProjectUsage, ReadManifestError, read_manifest};
