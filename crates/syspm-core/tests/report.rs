mod common;

use common::{canonical, fixture, write_file};
use std::fs;
use std::path::Path;
use std::process::Command;
use syspm_core::{MetadataStatus, ProjectReport, Rule, Severity, inspect_project};

/// Creates a project in `root`, with a model file in the `models` subdirectory.
fn write_project(root: &Path) {
    write_file(
        &root.join(".project.json"),
        r#"{"name": "model", "version": "0.1.0"}"#,
    );
    write_file(&root.join("models/A.sysml"), "package A;\n");
}

/// Writes a project to `root` with an unreadable directory inside, and inspects it. Returns
/// `None` if the directory can be read anyway, because the test is running as root.
#[cfg(unix)]
fn inspect_with_unreadable_directory(root: &Path) -> Option<ProjectReport> {
    use std::os::unix::fs::PermissionsExt;

    write_project(root);
    let locked = root.join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok();
    let report = inspect_project(&canonical(root));
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return None;
    }
    Some(report.unwrap())
}

#[cfg(unix)]
fn count_unreadable_diagnostics(report: &ProjectReport) -> usize {
    report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule == Rule::FileUnreadable)
        .count()
}

#[test]
fn reports_project_with_only_warnings() {
    let root = canonical(&fixture("discovery"));
    let report = inspect_project(&root).unwrap();
    assert_eq!(report.root, root);
    assert_eq!(report.manifest.name, "discovery");
    assert!(matches!(report.metadata, MetadataStatus::Parsed(_)));
    assert!(report.scan.is_complete());
    assert_eq!(report.scan.files.len(), 4);
    assert_eq!(report.diagnostics.len(), 3);
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning)
    );
    assert!(!report.has_errors());
}

#[test]
fn reports_project_with_missing_metadata() {
    let root = canonical(&fixture("controller"));
    let report = inspect_project(&root).unwrap();
    assert!(matches!(report.metadata, MetadataStatus::Missing));
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].rule, Rule::MetaMissing);
    assert!(!report.has_errors());
}

#[test]
fn reports_project_with_malformed_metadata() {
    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    write_file(&dir.path().join(".meta.json"), "{");
    let report = inspect_project(&canonical(dir.path())).unwrap();
    assert!(matches!(report.metadata, MetadataStatus::Failed(_)));
    assert_eq!(report.scan.files, ["models/A.sysml"]);
    // The file exists, so it must not be reported as missing.
    assert_eq!(report.diagnostics, []);
    assert!(report.has_errors());
}

#[test]
fn reports_project_with_metadata_validation_errors() {
    let root = canonical(&fixture("invalid-metadata"));
    let report = inspect_project(&root).unwrap();
    assert!(matches!(report.metadata, MetadataStatus::Parsed(_)));
    // Findings about both files, merged into one sorted list.
    assert_eq!(report.diagnostics.len(), 8);
    assert!(report.diagnostics.is_sorted());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    );
    assert!(report.has_errors());
}

#[cfg(unix)]
#[test]
fn reports_project_with_locked_directory_and_no_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let Some(report) = inspect_with_unreadable_directory(dir.path()) else {
        return; // Running as root: permissions are not enforced.
    };
    assert!(matches!(report.metadata, MetadataStatus::Missing));
    assert!(!report.scan.is_complete());
    assert_eq!(count_unreadable_diagnostics(&report), 1);
    assert!(report.has_errors());
}

#[cfg(unix)]
#[test]
fn reports_project_with_locked_directory_and_metadata() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".meta.json"),
        r#"{"index": {}, "created": "2026-10-31T12:00:00Z"}"#,
    );
    let Some(report) = inspect_with_unreadable_directory(dir.path()) else {
        return; // Running as root: permissions are not enforced.
    };
    assert!(matches!(report.metadata, MetadataStatus::Parsed(_)));
    assert!(!report.scan.is_complete());
    // Reported only once, even though the metadata was checked against the same scan.
    assert_eq!(count_unreadable_diagnostics(&report), 1);
    assert!(report.has_errors());
}

#[cfg(unix)]
#[test]
fn named_pipe_metadata_is_metadata_failure() {
    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());

    // Opening a FIFO blocks until a writer appears, so this test hangs if it is read.
    let pipe = dir.path().join(".meta.json");
    assert!(
        Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );

    let report = inspect_project(&canonical(dir.path())).unwrap();
    assert!(matches!(report.metadata, MetadataStatus::Failed(_)));
}

#[cfg(unix)]
#[test]
fn broken_metadata_symlink_is_metadata_failure() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    symlink(
        dir.path().join("non-existent"),
        dir.path().join(".meta.json"),
    )
    .unwrap();

    let report = inspect_project(&canonical(dir.path())).unwrap();
    assert!(matches!(report.metadata, MetadataStatus::Failed(_)));
}
