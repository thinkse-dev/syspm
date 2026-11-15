use std::path::{Path, PathBuf};
use syspm_core::{ReadProjectError, read_project};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

#[test]
fn reads_fixture_manifest() {
    let manifest = read_project(&fixture("common-interfaces/.project.json")).unwrap();
    assert_eq!(manifest.name, "common-interfaces");
    assert_eq!(manifest.version, "0.1.0");
    assert!(manifest.usage.is_empty());
}

#[test]
fn missing_manifest_is_io_error_naming_path() {
    let path = fixture("does-not-exist/.project.json");
    let error = read_project(&path).unwrap_err();
    assert!(matches!(error, ReadProjectError::Io { .. }));
    assert!(error.to_string().contains("does-not-exist"));
}

#[test]
fn error_message_does_not_repeat_its_source() {
    let error = read_project(&fixture("does-not-exist/.project.json")).unwrap_err();
    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn malformed_manifest_is_parse_error() {
    let dir = std::env::temp_dir().join(format!("syspm-core-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(".project.json");
    std::fs::write(&path, r#"{"name": "missing-version"}"#).unwrap();

    let error = read_project(&path).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();

    assert!(matches!(error, ReadProjectError::Parse { .. }));
}
