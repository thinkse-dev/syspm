use std::fs;
use std::path::Path;
use syspm_core::{InitError, init_project, read_manifest, validate_manifest};

#[test]
fn creates_manifest_that_reads_and_validates() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = init_project(dir.path(), Some("my-project"), "0.1.0").unwrap();
    assert_eq!(manifest_path, dir.path().join(".project.json"));
    let manifest = read_manifest(&manifest_path).unwrap();
    assert_eq!(manifest.name, "my-project");
    assert_eq!(manifest.version, "0.1.0");
    assert!(validate_manifest(&manifest).is_empty());
    let contents = fs::read_to_string(&manifest_path).unwrap();
    assert!(contents.ends_with('\n'), "{contents}");
}

#[test]
fn name_defaults_to_directory_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("my-project");
    fs::create_dir(&path).unwrap();
    let manifest_path = init_project(&path, None, "0.1.0").unwrap();
    assert_eq!(manifest_path, path.join(".project.json"));
    let manifest = read_manifest(&manifest_path).unwrap();
    assert_eq!(manifest.name, "my-project");
}

#[test]
fn default_name_resolves_parent_components() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("my-project/sub");
    fs::create_dir_all(&path).unwrap();
    let manifest_path = init_project(&path.join(".."), None, "0.1.0").unwrap();
    let manifest = read_manifest(&manifest_path).unwrap();
    assert_eq!(manifest.name, "my-project");
}

#[test]
fn empty_name_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let error = init_project(dir.path(), Some(""), "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::EmptyName), "{error}");
    assert!(!dir.path().join(".project.json").exists());
}

#[cfg(unix)]
#[test]
fn filesystem_root_has_no_default_name() {
    let error = init_project(Path::new("/"), None, "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::NoDefaultName { .. }), "{error}");
}

#[test]
fn project_above_directory_does_not_prevent_init() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".project.json"), "original").unwrap();
    let child = dir.path().join("child");
    fs::create_dir(&child).unwrap();
    let manifest_path = init_project(&child, Some("my-project"), "0.1.0").unwrap();
    assert_eq!(manifest_path, child.join(".project.json"));
    let contents = fs::read_to_string(dir.path().join(".project.json")).unwrap();
    assert_eq!(contents, "original");
}

#[test]
fn existing_manifest_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".project.json"), "original").unwrap();
    let error = init_project(dir.path(), Some("my-project"), "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::AlreadyExists { .. }), "{error}");
    let contents = fs::read_to_string(dir.path().join(".project.json")).unwrap();
    assert_eq!(contents, "original");
}

#[cfg(unix)]
#[test]
fn broken_manifest_symlink_is_not_followed() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    symlink(
        dir.path().join("non-existent"),
        dir.path().join(".project.json"),
    )
    .unwrap();
    let error = init_project(dir.path(), Some("my-project"), "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::AlreadyExists { .. }), "{error}");
    assert!(!dir.path().join("non-existent").exists());
}

#[test]
fn manifest_path_that_is_a_directory_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".project.json")).unwrap();
    let error = init_project(dir.path(), Some("my-project"), "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::AlreadyExists { .. }), "{error}");
}

#[test]
fn missing_directory_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let error = init_project(
        &dir.path().join("non-existent"),
        Some("my-project"),
        "0.1.0",
    )
    .unwrap_err();
    assert!(matches!(error, InitError::Io { .. }), "{error}");
    assert!(
        error
            .to_string()
            .contains(dir.path().join("non-existent").to_str().unwrap()),
        "{error}"
    );
}

#[test]
fn missing_directory_with_default_name_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let error = init_project(&dir.path().join("non-existent"), None, "0.1.0").unwrap_err();
    assert!(matches!(error, InitError::Io { .. }), "{error}");
    assert!(
        error
            .to_string()
            .contains(dir.path().join("non-existent").to_str().unwrap()),
        "{error}"
    );
}
