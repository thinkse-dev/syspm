//! The library behind syspm. All of the package-management logic lives here, so any front end
//! can use it.
//!
//! So far it reads and validates a project's `.project.json` and `.meta.json`, finds the project
//! root, lists the model files, gathers all of that into one report for `inspect`, and creates
//! the manifest of a new project. Everything to do with dependencies comes later.

mod discover;
mod init;
mod interchange;
mod report;
mod validate;

pub use discover::{
    DiscoverError, ModelFileScan, find_project_root, list_model_files, scan_model_files,
};
pub use init::{InitError, init_project};
pub use interchange::{
    AdditionalProperties, Checksum, ProjectManifest, ProjectMetadata, ProjectUsage,
    ReadManifestError, ReadMetadataError, read_manifest, read_metadata,
};

pub use report::{MetadataStatus, ProjectReport, inspect_project};
pub use validate::{Diagnostic, Rule, Severity, validate_manifest, validate_metadata};
