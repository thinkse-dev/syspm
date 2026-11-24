//! The library behind syspm. All of the package-management logic lives here, so any front end
//! can use it.
//!
//! So far it reads and validates a project's `.project.json` and `.meta.json`, finds the project
//! root, and lists the model files. Everything to do with dependencies comes later.

mod discover;
mod interchange;
mod validate;

pub use discover::{
    DiscoverError, ModelFileScan, find_project_root, list_model_files, scan_model_files,
};

pub use interchange::{
    Checksum, ProjectManifest, ProjectMetadata, ProjectUsage, ReadManifestError, ReadMetadataError,
    read_manifest, read_metadata,
};

pub use validate::{Diagnostic, Rule, Severity, validate_manifest, validate_metadata};
