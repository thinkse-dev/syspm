mod common;

use common::fixture;
use std::path::{Path, PathBuf};
use syspm_core::{ProjectManifest, ReadManifestError, read_manifest};

/// Writes `contents` to `.project.json` in a fresh temporary directory and reads it back. The
/// directory goes away when this returns.
fn read_temp_manifest(contents: &str) -> (PathBuf, Result<ProjectManifest, ReadManifestError>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".project.json");
    std::fs::write(&path, contents).unwrap();

    let result = read_manifest(&path);
    (path, result)
}

/// Checks that `result` is a `Parse` error whose message names `path`, with the JSON error in the
/// source and not repeated in the message.
fn assert_parse_error_naming(path: &Path, result: Result<ProjectManifest, ReadManifestError>) {
    let error = result.unwrap_err();
    assert!(matches!(error, ReadManifestError::Parse { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));

    let source = std::error::Error::source(&error).expect("parse error has a source");
    assert!(source.downcast_ref::<serde_json::Error>().is_some());
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn reads_fixture_manifest() {
    let manifest = read_manifest(&fixture("common-interfaces/.project.json")).unwrap();
    assert_eq!(manifest.name, "common-interfaces");
    assert_eq!(manifest.version, "0.1.0");
    assert!(manifest.usage.is_empty());
}

#[test]
fn reads_every_table_12_property() {
    let manifest = read_manifest(&fixture("all-properties/.project.json")).unwrap();
    assert_eq!(manifest.name, "all-properties");
    assert_eq!(
        manifest.description.as_deref(),
        Some("Illustrative manifest fixture with every KerML 1.0 Table 12 property")
    );
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.license.as_deref(), Some("Apache-2.0 OR MIT"));
    assert_eq!(
        manifest.maintainer,
        [
            "Example Maintainer <maintainer@example.invalid>",
            "Second Maintainer"
        ]
    );
    assert_eq!(
        manifest.website.as_deref(),
        Some("https://example.invalid/all-properties")
    );
    assert_eq!(manifest.topic, ["fixture", "metadata"]);
    assert_eq!(manifest.usage.len(), 1);
    assert_eq!(
        manifest.usage[0].resource,
        "https://example.invalid/common-interfaces.kpar"
    );
    assert_eq!(
        manifest.usage[0].version_constraint.as_deref(),
        Some("^0.1.0")
    );
}

#[test]
fn reads_usages_with_and_without_constraints() {
    let manifest = read_manifest(&fixture("controller/.project.json")).unwrap();
    let usages: Vec<_> = manifest
        .usage
        .iter()
        .map(|usage| (usage.resource.as_str(), usage.version_constraint.as_deref()))
        .collect();
    assert_eq!(
        usages,
        [
            (
                "https://example.invalid/common-interfaces.kpar",
                Some("^0.1.0")
            ),
            ("https://example.invalid/units.kpar", None),
        ]
    );
}

#[test]
fn omitted_optional_properties_use_defaults() {
    let (_, result) = read_temp_manifest(r#"{"name": "required-only", "version": "0.1.0"}"#);
    let manifest = result.unwrap();
    assert_eq!(manifest.name, "required-only");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.description, None);
    assert_eq!(manifest.license, None);
    assert!(manifest.maintainer.is_empty());
    assert_eq!(manifest.website, None);
    assert!(manifest.topic.is_empty());
    assert!(manifest.usage.is_empty());
}

#[test]
fn unknown_properties_are_accepted() {
    let (_, result) = read_temp_manifest(
        r#"{
            "name": "extended",
            "version": "0.1.0",
            "x-tool-setting": {"nested": true},
            "usage": [{"resource": "https://example.invalid/a.kpar", "extra": 1}]
        }"#,
    );
    let manifest = result.unwrap();
    assert_eq!(manifest.name, "extended");
    assert_eq!(manifest.usage[0].resource, "https://example.invalid/a.kpar");
}

#[test]
fn repeated_property_is_parse_error() {
    for contents in [
        r#"{"name": "a", "name": "b", "version": "1"}"#,
        r#"{"name": "n", "version": "1", "usage": [{"resource": "https://example.invalid/a", "resource": "https://example.invalid/b"}]}"#,
    ] {
        let (path, result) = read_temp_manifest(contents);
        assert_parse_error_naming(&path, result);
    }
}

#[test]
fn missing_manifest_is_io_error_naming_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".project.json");

    let error = read_manifest(&path).unwrap_err();
    assert!(matches!(error, ReadManifestError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));
}

#[test]
fn error_message_does_not_repeat_its_source() {
    let dir = tempfile::tempdir().unwrap();
    let error = read_manifest(&dir.path().join(".project.json")).unwrap_err();
    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn missing_required_property_is_parse_error() {
    for contents in [r#"{"name": "no-version"}"#, r#"{"version": "0.1.0"}"#] {
        let (path, result) = read_temp_manifest(contents);
        assert_parse_error_naming(&path, result);
    }
}

#[test]
fn wrong_property_type_is_parse_error() {
    for property in [
        r#""maintainer": "Not An Array""#,
        r#""topic": [1, 2]"#,
        r#""description": 3"#,
        r#""usage": {"resource": "https://example.invalid/a.kpar"}"#,
        r#""usage": [{"resource": "https://example.invalid/a.kpar", "versionConstraint": 1}]"#,
    ] {
        let (path, result) = read_temp_manifest(&format!(
            r#"{{"name": "wrong-type", "version": "0.1.0", {property}}}"#
        ));
        assert_parse_error_naming(&path, result);
    }
}

#[test]
fn manifest_as_array_is_parse_error() {
    // Without the object check, serde would match these to the fields by position.
    let (path, result) = read_temp_manifest(r#"["n", "a description", "1.0"]"#);
    assert_parse_error_naming(&path, result);
}

#[test]
fn usage_as_array_is_parse_error() {
    let (path, result) = read_temp_manifest(
        r#"{"name": "n", "version": "1", "usage": [["https://example.invalid/a.kpar"]]}"#,
    );
    assert_parse_error_naming(&path, result);
}

#[test]
fn null_optional_string_is_parse_error() {
    for property in [
        r#""description": null"#,
        r#""license": null"#,
        r#""website": null"#,
        r#""usage": [{"resource": "https://example.invalid/a.kpar", "versionConstraint": null}]"#,
    ] {
        let (path, result) = read_temp_manifest(&format!(
            r#"{{"name": "n", "version": "0.1.0", {property}}}"#
        ));
        assert_parse_error_naming(&path, result);
    }
}

#[cfg(unix)]
#[test]
fn manifest_symlink_is_io_error_even_to_a_valid_file() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    std::fs::write(
        outside.path().join(".project.json"),
        r#"{"name": "model", "version": "0.1.0"}"#,
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    symlink(
        outside.path().join(".project.json"),
        dir.path().join(".project.json"),
    )
    .unwrap();

    let error = read_manifest(&dir.path().join(".project.json")).unwrap_err();
    assert!(matches!(error, ReadManifestError::Io { .. }));
    assert!(error.to_string().contains("cannot read project manifest"));
}

#[test]
fn directory_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".project.json");
    std::fs::create_dir(&path).unwrap();

    let error = read_manifest(&path).unwrap_err();
    assert!(matches!(error, ReadManifestError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));
}
