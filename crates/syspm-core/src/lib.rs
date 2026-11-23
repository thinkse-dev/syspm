//! The library behind syspm. All of the package-management logic lives here, so any front end
//! can use it.
//!
//! So far it reads a project's `.project.json` and `.meta.json`. Validation and everything to do
//! with dependencies come later.

mod interchange;

pub use interchange::{
    Checksum, ProjectManifest, ProjectMetadata, ProjectUsage, ReadManifestError, ReadMetadataError,
    read_manifest, read_metadata,
};
