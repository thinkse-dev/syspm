//! Gathers what can be learned about a project into one report.
//!
//! An inspection reports whatever it can obtain safely, so a problem with `.meta.json` or with
//! listing model files is recorded in the report instead of ending the inspection. Only a manifest
//! that can't be read leaves nothing to report.

use crate::interchange::{PROJECT_MANIFEST, PROJECT_METADATA};
use crate::validate::{
    listing_diagnostics, missing_metadata_diagnostic, validate_metadata_with_scan,
};
use crate::{
    Diagnostic, ModelFileScan, ProjectManifest, ProjectMetadata, ReadManifestError,
    ReadMetadataError, Severity, read_manifest, read_metadata, scan_model_files, validate_manifest,
};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The status of a project's `.meta.json`: whether reading it gave metadata, found none, or failed.
#[derive(Debug)]
pub enum MetadataStatus {
    /// `.meta.json` was read and parsed.
    Parsed(ProjectMetadata),
    /// The project has no `.meta.json`, which is a warning rather than a failure.
    Missing,
    /// `.meta.json` exists but couldn't be read or parsed.
    Failed(ReadMetadataError),
}

impl MetadataStatus {
    fn read(path: &Path) -> Self {
        match read_metadata(path) {
            Ok(metadata) => MetadataStatus::Parsed(metadata),
            Err(ReadMetadataError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => {
                MetadataStatus::Missing
            }
            Err(error) => MetadataStatus::Failed(error),
        }
    }
}

/// Everything an inspection learned about a project.
#[derive(Debug)]
pub struct ProjectReport {
    /// The project's root directory, for display. Not part of the project's metadata.
    pub root: PathBuf,
    pub manifest: ProjectManifest,
    pub metadata: MetadataStatus,
    pub scan: ModelFileScan,
    /// Every finding about the project, sorted. A failed read of `.meta.json` isn't one of them;
    /// that is recorded in `metadata`.
    pub diagnostics: Vec<Diagnostic>,
}

impl ProjectReport {
    /// Whether the project has a problem that should fail the inspection: an error diagnostic, or
    /// a `.meta.json` that couldn't be read. Warnings don't count.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
            || matches!(self.metadata, MetadataStatus::Failed(_))
    }
}

/// Inspects the project in `root`, a directory such as [`crate::find_project_root`] returns.
///
/// The project is scanned once, so the listed files and the findings about them always agree.
/// Returns an error only if the manifest can't be read. See [`ProjectReport::has_errors`] for
/// everything else.
pub fn inspect_project(root: &Path) -> Result<ProjectReport, ReadManifestError> {
    let manifest = read_manifest(&root.join(PROJECT_MANIFEST))?;
    let metadata = MetadataStatus::read(&root.join(PROJECT_METADATA));
    let scan = scan_model_files(root);

    let mut diagnostics = validate_manifest(&manifest);
    diagnostics.extend(listing_diagnostics(root, &scan));
    diagnostics.extend(match &metadata {
        MetadataStatus::Parsed(parsed) => validate_metadata_with_scan(root, parsed, &scan),
        MetadataStatus::Missing => vec![missing_metadata_diagnostic()],
        MetadataStatus::Failed(_) => Vec::new(),
    });
    diagnostics.sort();

    Ok(ProjectReport {
        root: root.to_path_buf(),
        manifest,
        metadata,
        scan,
        diagnostics,
    })
}
