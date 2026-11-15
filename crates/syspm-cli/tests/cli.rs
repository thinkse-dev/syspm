use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn syspm(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_syspm"))
        .args(args)
        .output()
        .expect("failed to run syspm binary")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
    for command in ["init", "install", "update"] {
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
fn inspect_reads_project_directory() {
    let dir = fixture("common-interfaces");
    let output = syspm(&["inspect", dir.to_str().expect("fixture path is UTF-8")]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Project: common-interfaces"), "{text}");
    assert!(text.contains("Version: 0.1.0"), "{text}");
    assert!(text.contains("Declared usages: 0"), "{text}");
}

#[test]
fn inspect_lists_usages_with_optional_constraints() {
    let dir = fixture("controller");
    let output = syspm(&["inspect", dir.to_str().expect("fixture path is UTF-8")]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Project: controller\n\
         Version: 0.2.0\n\
         Declared usages: 2\n  \
         https://example.invalid/common-interfaces.kpar (^0.1.0)\n  \
         https://example.invalid/units.kpar\n"
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
fn inspect_missing_manifest_fails_with_path() {
    let dir = fixture("does-not-exist");
    let output = syspm(&["inspect", dir.to_str().expect("fixture path is UTF-8")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with("syspm: cannot read project manifest"),
        "{text}"
    );
    assert!(text.contains("does-not-exist"), "{text}");
    // The OS-level cause is appended once, after the context.
    assert_eq!(text.matches("os error").count(), 1, "{text}");
}

#[test]
fn inspect_invalid_manifest_reports_parse_cause() {
    let dir = std::env::temp_dir().join(format!("syspm-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join(".project.json"), r#"{"name": "x"}"#).expect("write manifest");

    let output = syspm(&["inspect", dir.to_str().expect("temp path is UTF-8")]);
    std::fs::remove_dir_all(&dir).expect("remove temp dir");

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    let text = stderr(&output);
    assert!(
        text.starts_with("syspm: invalid project manifest"),
        "{text}"
    );
    assert!(text.contains("missing field `version`"), "{text}");
}
