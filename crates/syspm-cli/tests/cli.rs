use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn syspm(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_syspm"))
        .args(args)
        .output()
        .expect("failed to run syspm binary")
}

fn syspm_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_syspm"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run syspm binary")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).expect("fixture directory exists")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Creates a file, along with any missing parent directories.
fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// Creates a project in `root`, with a model file in the `models` subdirectory.
fn write_project(root: &Path) {
    write_file(
        &root.join(".project.json"),
        r#"{"name": "model", "version": "0.1.0"}"#,
    );
    write_file(&root.join("models/A.sysml"), "package A;\n");
}

/// Creates a project whose `.project.json` is a symbolic link to another project's valid manifest.
/// Returns the project, then the other project, which must stay alive for the link to stay valid.
#[cfg(unix)]
fn project_with_linked_manifest() -> (tempfile::TempDir, tempfile::TempDir) {
    let outside = tempfile::tempdir().expect("create temp dir");
    write_project(outside.path());
    let dir = tempfile::tempdir().expect("create temp dir");
    std::os::unix::fs::symlink(
        outside.path().join(".project.json"),
        dir.path().join(".project.json"),
    )
    .expect("create symbolic link");
    (dir, outside)
}

/// Asserts that reading a manifest failed the regular-file check, with nothing reported.
#[cfg(unix)]
fn assert_manifest_rejected(output: &Output) {
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(output).is_empty());
    let errors = stderr(output);
    assert!(
        errors.contains("syspm: cannot read project manifest"),
        "{errors}"
    );
    assert!(errors.contains("not a regular file"), "{errors}");
}

#[test]
fn help_describes_application_and_commands() {
    let output = syspm(&["--help"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("SysML v2 and KerML"), "{text}");
    for command in ["init", "inspect", "install", "update"] {
        assert!(text.contains(command), "missing `{command}` in:\n{text}");
    }
}

#[test]
fn version_prints_package_version() {
    let output = syspm(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        stdout(&output).trim(),
        format!("syspm {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn missing_subcommand_is_usage_error() {
    let output = syspm(&[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("Usage: syspm"));
}

#[test]
fn unknown_subcommand_is_usage_error_on_stderr() {
    let output = syspm(&["frobnicate"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(
        text.contains("unrecognized subcommand 'frobnicate'"),
        "{text}"
    );
}

#[test]
fn unknown_flag_is_usage_error_on_stderr() {
    let output = syspm(&["inspect", "--bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(text.contains("unexpected argument '--bogus'"), "{text}");
}

#[test]
fn unimplemented_commands_fail_clearly() {
    for command in ["install", "update"] {
        let output = syspm(&[command]);
        assert_eq!(output.status.code(), Some(1), "`{command}` exit status");
        assert!(stdout(&output).is_empty());
        assert_eq!(
            stderr(&output).trim(),
            format!("syspm: `{command}` is not implemented yet")
        );
    }
}

#[test]
fn init_help_documents_defaults() {
    let output = syspm(&["init", "--help"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("[default: .]"), "{text}");
    assert!(text.contains("[default: 0.1.0]"), "{text}");
    assert!(text.contains("[default: the directory's name]"), "{text}");
}

#[test]
fn init_creates_valid_project() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let project = dir.path().join("my-model");
    fs::create_dir(&project).unwrap();

    let output = syspm_in(&project, &["init"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(".project.json"));
    let manifest = syspm_core::read_manifest(&project.join(".project.json")).unwrap();
    assert_eq!(manifest.name, "my-model");
    assert_eq!(manifest.version, "0.1.0");
    assert!(syspm_core::validate_manifest(&manifest).is_empty());
    assert_eq!(fs::read_dir(&project).unwrap().count(), 1);
}

#[test]
fn init_accepts_path_and_flags() {
    let dir = tempfile::tempdir().expect("create temp dir");

    let output = syspm(&[
        "init",
        dir.path().to_str().expect("temp path is UTF-8"),
        "--name",
        "custom",
        "--version",
        "2.0.0",
    ]);

    assert!(output.status.success(), "{}", stderr(&output));
    let manifest = syspm_core::read_manifest(&dir.path().join(".project.json")).unwrap();
    assert_eq!(manifest.name, "custom");
    assert_eq!(manifest.version, "2.0.0");
}

#[test]
fn init_rejects_empty_name() {
    let dir = tempfile::tempdir().expect("create temp dir");

    let output = syspm_in(dir.path(), &["init", "--name", ""]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output).trim(), "syspm: project name is empty");
    assert!(!dir.path().join(".project.json").exists());
}

#[cfg(unix)]
#[test]
fn init_without_usable_directory_name_asks_for_name() {
    let output = syspm(&["init", "/"]);

    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.contains("--name"), "{text}");
}

#[test]
fn init_keeps_existing_manifest() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let manifest = dir.path().join(".project.json");
    fs::write(&manifest, "original").unwrap();

    let output = syspm_in(dir.path(), &["init"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(text.contains(".project.json"), "{text}");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), "original");
}

#[test]
fn init_missing_directory_fails() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let missing = dir.path().join("missing");
    let missing = missing.to_str().expect("temp path is UTF-8");

    let output = syspm(&["init", missing]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(text.contains(missing), "{text}");
}

#[cfg(unix)]
#[test]
fn init_unwritable_directory_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("create temp dir");
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let writable = fs::write(locked.join("probe"), "").is_ok();
    let locked_path = locked.to_str().expect("temp path is UTF-8");
    let output = syspm(&["init", locked_path]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if writable {
        return; // Running as root: permissions are not enforced.
    }

    assert_eq!(output.status.code(), Some(1));
    let text = stderr(&output);
    assert!(text.contains(locked_path), "{text}");
    assert!(!locked.join(".project.json").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn init_removes_partial_manifest_when_write_fails() {
    let dir = tempfile::tempdir().expect("create temp dir");

    // A file size limit of zero makes the write fail after the file is created. Ignoring SIGXFSZ
    // makes the write return an error instead of killing the process, and the setting carries
    // over to `syspm`.
    let output = Command::new("sh")
        .arg("-c")
        .arg(r#"trap '' XFSZ; ulimit -f 0; exec "$0" init --name model"#)
        .arg(env!("CARGO_BIN_EXE_syspm"))
        .current_dir(dir.path())
        .output()
        .expect("failed to run syspm binary");

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.contains("cannot write project manifest"), "{text}");
    assert!(!dir.path().join(".project.json").exists());
}

#[test]
fn inspect_defaults_to_current_directory() {
    let dir = canonical(&fixture("discovery/models/deep"));
    let root = canonical(&fixture("discovery"));
    let output = syspm_in(&dir, &["inspect"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.starts_with(&format!("Root: {}\n", root.display())),
        "{text}"
    );
}

#[test]
fn inspect_discovers_project_from_nested_directory() {
    let root = canonical(&fixture("discovery"));
    let dir = fixture("discovery/models/deep");

    let output = syspm(&["inspect", dir.to_str().expect("fixture path is UTF-8")]);

    assert!(output.status.success(), "{}", stderr(&output));
    let errors = stderr(&output);
    assert!(errors.is_empty(), "{errors}");
    assert_eq!(
        stdout(&output),
        format!(
            "Root: {}\n\
             Project: discovery\n\
             Version: 0.1.0\n\
             Description: Illustrative project tree for project discovery tests\n\
             Declared usages: 0\n\
             Metadata:\n  \
             Created: 2026-10-31T12:00:00Z\n  \
             Index entries: 1\n\
             Model files: 4\n  \
             Core.kerml\n  \
             Root.sysml\n  \
             models/Nested.sysml\n  \
             models/deep/Deep.kerml\n\
             Validation: 0 errors, 3 warnings\n  \
             warning[index-unreferenced-file] Core.kerml: model file is not referred to by the index\n  \
             warning[index-unreferenced-file] models/Nested.sysml: model file is not referred to by the index\n  \
             warning[index-unreferenced-file] models/deep/Deep.kerml: model file is not referred to by the index\n",
            root.display()
        )
    );
}

#[test]
fn inspect_accepts_manifest_file_path() {
    let manifest = fixture("common-interfaces/.project.json");
    let output = syspm(&["inspect", manifest.to_str().expect("fixture path is UTF-8")]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Project: common-interfaces"));
}

#[test]
fn inspect_accepts_relative_manifest_path() {
    let output = syspm_in(&fixture("common-interfaces"), &["inspect", ".project.json"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("Project: common-interfaces"));
}

#[test]
fn inspect_missing_path_fails_naming_path() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let dir = temp.path().join("does-not-exist");
    let output = syspm(&["inspect", dir.to_str().expect("temp path is UTF-8")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    let expected = format!("syspm: cannot read {}", dir.display());
    assert!(text.starts_with(&expected), "{text}");
    // The OS-level cause is appended once, after the context.
    assert_eq!(text.matches("os error").count(), 1, "{text}");
}

#[test]
fn inspect_outside_any_project_fails() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let output = syspm(&["inspect", dir.path().to_str().expect("temp path is UTF-8")]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with("syspm: no project found at or above "),
        "{text}"
    );
}

#[test]
fn inspect_reads_project_directory() {
    let root = canonical(&fixture("common-interfaces"));
    let output = syspm(&["inspect", root.to_str().expect("fixture path is UTF-8")]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "Root: {}\n\
             Project: common-interfaces\n\
             Version: 0.1.0\n\
             Description: Illustrative manifest fixture; not a complete interchange project\n\
             Declared usages: 0\n\
             Metadata:\n  \
             Created: 2026-10-30T12:00:00Z\n  \
             Index entries: 1\n\
             Model files: 1\n  \
             CommonInterfaces.sysml\n\
             Validation: 0 errors, 0 warnings\n",
            root.display()
        )
    );
}

#[test]
fn inspect_reports_every_manifest_property() {
    let root = canonical(&fixture("all-properties"));
    let output = syspm(&["inspect", root.to_str().expect("fixture path is UTF-8")]);

    assert!(output.status.success(), "{}", stderr(&output));
    let errors = stderr(&output);
    assert!(errors.is_empty(), "{errors}");
    assert_eq!(
        stdout(&output),
        format!(
            "Root: {}\n\
             Project: all-properties\n\
             Version: 1.0.0\n\
             Description: Illustrative manifest fixture with every KerML 1.0 Table 12 property\n\
             License: Apache-2.0 OR MIT\n\
             Maintainers: Example Maintainer <maintainer@example.invalid>, Second Maintainer\n\
             Website: https://example.invalid/all-properties\n\
             Topics: fixture, metadata\n\
             Declared usages: 1\n  \
             https://example.invalid/common-interfaces.kpar (^0.1.0)\n\
             Metadata:\n  \
             Created: 2026-10-30T12:00:00Z\n  \
             Metamodel: https://www.omg.org/spec/SysML/20250201\n  \
             Includes derived: false\n  \
             Includes implied: true\n  \
             Index entries: 2\n\
             Model files: 2\n  \
             AllProperties.sysml\n  \
             Extras.sysml\n\
             Validation: 0 errors, 0 warnings\n",
            root.display()
        )
    );
}

#[test]
fn inspect_lists_usages_with_optional_constraints() {
    let dir = canonical(&fixture("controller"));
    let output = syspm(&["inspect", dir.to_str().expect("fixture path is UTF-8")]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "Root: {}\n\
             Project: controller\n\
             Version: 0.2.0\n\
             Description: Illustrative manifest fixture with usages; not a complete interchange project\n\
             Declared usages: 2\n  \
             https://example.invalid/common-interfaces.kpar (^0.1.0)\n  \
             https://example.invalid/units.kpar\n\
             Metadata: none\n\
             Model files: 0\n\
             Validation: 0 errors, 1 warning\n  \
             warning[meta-missing] .meta.json: project has no metadata file\n",
            dir.display()
        )
    );
}

#[test]
fn inspect_accepts_initialized_project() {
    let dir = tempfile::tempdir().expect("create temp dir");

    let init = syspm_in(dir.path(), &["init"]);
    assert!(init.status.success(), "{}", stderr(&init));

    let inspect = syspm_in(dir.path(), &["inspect"]);
    assert!(inspect.status.success(), "{}", stderr(&inspect));
}

#[test]
fn inspect_invalid_manifest_reports_parse_cause() {
    let dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(dir.path().join(".project.json"), r#"{"name": "x"}"#).expect("write manifest");

    let output = syspm(&["inspect", dir.path().to_str().expect("temp path is UTF-8")]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with("syspm: invalid project manifest"),
        "{text}"
    );
    assert!(text.contains("missing field `version`"), "{text}");
}

#[cfg(unix)]
#[test]
fn inspect_broken_manifest_symlink_fails() {
    use std::os::unix::fs::symlink;

    let parent = tempfile::tempdir().unwrap();
    write_project(parent.path());

    let child = parent.path().join("child");
    fs::create_dir(&child).unwrap();

    symlink(child.join("non-existent"), child.join(".project.json")).unwrap();

    let output = syspm_in(&child, &["inspect"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let errors = stderr(&output);
    assert!(
        errors.starts_with("syspm: cannot read project manifest"),
        "{errors}"
    );
}

#[cfg(unix)]
#[test]
fn inspect_explicit_manifest_symlink_fails() {
    let (dir, _outside) = project_with_linked_manifest();
    let manifest = dir.path().join(".project.json");

    let output = syspm(&["inspect", manifest.to_str().expect("temp path is UTF-8")]);

    assert_manifest_rejected(&output);
}

#[cfg(unix)]
#[test]
fn inspect_relative_manifest_symlink_fails() {
    let (dir, _outside) = project_with_linked_manifest();

    let output = syspm_in(dir.path(), &["inspect", ".project.json"]);

    assert_manifest_rejected(&output);
}

#[test]
fn inspect_invalid_metadata_reports_errors_and_fails() {
    let root = fixture("invalid-metadata");
    let output = syspm(&["inspect", root.to_str().expect("fixture path is UTF-8")]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("Validation: 7 errors, 1 warning"), "{text}");
    assert!(text.contains("error[checksum-mismatch] .meta.json: checksum entry `Indexed.sysml` does not match the file"), "{text}");
    assert_eq!(
        stderr(&output).trim(),
        "syspm: project has validation errors"
    );
}

#[test]
fn inspect_malformed_metadata_prints_partial_report() {
    let dir = tempfile::tempdir().expect("create temp dir");
    write_project(dir.path());
    write_file(&dir.path().join(".meta.json"), "{");
    let output = syspm(&["inspect", dir.path().to_str().expect("temp path is UTF-8")]);
    assert_eq!(output.status.code(), Some(1));
    let report = stdout(&output);
    assert!(report.contains("Project:"), "{report}");
    assert!(report.contains("Metadata: unreadable"), "{report}");
    assert!(
        report.contains("Validation: 0 errors, 0 warnings (partial)"),
        "{report}"
    );
    let errors = stderr(&output);
    assert!(
        errors.starts_with("syspm: invalid project metadata"),
        "{errors}"
    );
}

#[cfg(unix)]
#[test]
fn inspect_reports_partial_scan() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("create temp dir");
    write_project(dir.path());
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok();
    let output = syspm(&["inspect", dir.path().to_str().expect("temp path is UTF-8")]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // Running as root: permissions are not enforced.
    }
    assert_eq!(output.status.code(), Some(1));
    let report = stdout(&output);
    assert!(report.contains("Metadata: none"), "{report}");
    assert!(report.contains("Model files: 1 (partial)"), "{report}");
    assert!(
        report.contains("error[file-unreadable] locked: cannot list model files"),
        "{report}"
    );

    assert_eq!(
        stderr(&output).trim(),
        "syspm: project has validation errors"
    );
}
