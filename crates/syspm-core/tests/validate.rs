mod common;

use common::{fixture, write_file};
use std::fs;
use std::path::Path;
use syspm_core::{
    Diagnostic, ProjectManifest, ProjectMetadata, Severity, read_manifest, read_metadata,
    validate_manifest, validate_metadata,
};
use tempfile::TempDir;

/// The SHA-256 of `package A;\n`, which is what `project` writes to `A.sysml`.
const A_SHA256: &str = "d57b369878381818abf6e137575d170ed230aa147639a20fe3b870516f0a05f9";
const CREATED: &str = "2026-10-31T12:00:00Z";

/// A project containing `.project.json` and a model file `A.sysml`.
fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_file(&dir.path().join(".project.json"), "{}");
    write_file(&dir.path().join("A.sysml"), "package A;\n");
    dir
}

fn manifest(contents: &str) -> ProjectManifest {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".project.json");
    fs::write(&path, contents).unwrap();
    read_manifest(&path).unwrap()
}

/// Writes `contents` as the project's `.meta.json`, then reads and validates it.
fn validate(root: &Path, contents: &str) -> Vec<Diagnostic> {
    let path = root.join(".meta.json");
    fs::write(&path, contents).unwrap();
    let metadata: ProjectMetadata = read_metadata(&path).unwrap();
    validate_metadata(root, Some(&metadata))
}

/// `.meta.json` contents with the given `index` and `checksum` objects.
fn metadata_json(index: &str, checksum: &str) -> String {
    format!(r#"{{"index": {index}, "created": "{CREATED}", "checksum": {checksum}}}"#)
}

fn metadata_json_with_a_indexed(checksum: &str) -> String {
    metadata_json(r#"{"A": "A.sysml"}"#, checksum)
}

fn sha256_checksum(path: &str, value: &str) -> String {
    format!(r#"{{"{path}": {{"value": "{value}", "algorithm": "SHA256"}}}}"#)
}

/// The rule identifiers of `diagnostics`, in order.
fn rule_ids(diagnostics: &[Diagnostic]) -> Vec<&'static str> {
    diagnostics.iter().map(|d| d.rule.id()).collect()
}

/// Asserts that the only diagnostic is `rule`, reported against `path` with `severity`.
fn assert_single(diagnostics: &[Diagnostic], rule: &str, path: &str, severity: Severity) {
    assert_eq!(rule_ids(diagnostics), [rule], "{diagnostics:#?}");
    assert_eq!(diagnostics[0].path, path);
    assert_eq!(diagnostics[0].severity, severity);
    assert!(!diagnostics[0].message.is_empty());
}

#[test]
fn valid_fixture_projects_produce_no_diagnostics() {
    for name in [
        "all-properties",
        "common-interfaces",
        "additional-properties",
        "toolkit-archive",
    ] {
        let root = fixture(name);
        let manifest = read_manifest(&root.join(".project.json")).unwrap();
        let metadata = read_metadata(&root.join(".meta.json")).unwrap();
        assert_eq!(validate_manifest(&manifest), [], "{name}");
        assert_eq!(validate_metadata(&root, Some(&metadata)), [], "{name}");
    }
}

#[test]
fn project_without_metadata_only_warns() {
    let root = fixture("controller");
    let manifest = read_manifest(&root.join(".project.json")).unwrap();
    assert_eq!(validate_manifest(&manifest), []);
    assert_eq!(rule_ids(&validate_metadata(&root, None)), ["meta-missing"]);
}

#[test]
fn every_problem_is_reported_in_deterministic_order() {
    let root = fixture("invalid-metadata");
    let manifest = read_manifest(&root.join(".project.json")).unwrap();
    assert_eq!(
        rule_ids(&validate_manifest(&manifest)),
        ["project-website-iri", "project-usage-iri"]
    );

    let metadata = read_metadata(&root.join(".meta.json")).unwrap();
    let diagnostics = validate_metadata(&root, Some(&metadata));
    assert_eq!(
        rule_ids(&diagnostics),
        [
            "meta-created-format",
            "meta-metamodel-iri",
            "index-path-syntax",
            "index-file-missing",
            "checksum-mismatch",
            "index-unreferenced-file",
        ]
    );
    assert_eq!(diagnostics[5].path, "Unindexed.sysml");
    assert_eq!(validate_metadata(&root, Some(&metadata)), diagnostics);
}

#[test]
fn diagnostics_sort_by_path_then_rule() {
    let dir = project();
    write_file(&dir.path().join("Z.sysml"), "");
    write_file(&dir.path().join("B.sysml"), "");
    let diagnostics = validate(dir.path(), &metadata_json("{}", "{}"));
    let files: Vec<&str> = diagnostics.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(files, ["A.sysml", "B.sysml", "Z.sysml"]);
}

#[test]
fn valid_iris_are_accepted() {
    let manifest = manifest(
        r#"{
            "name": "n",
            "version": "1",
            "website": "https://example.invalid/caf%C3%A9?q=1#top",
            "usage": [
                {"resource": "urn:kpar:foo"},
                {"resource": "https://example.invalid/modèle.kpar"},
                {"resource": "https://[2001:db8::1]/a.kpar"},
                {"resource": "https://example.invalid/a.kpar#part"}
            ]
        }"#,
    );
    assert_eq!(validate_manifest(&manifest), []);
}

#[test]
fn invalid_website_is_reported() {
    for website in [
        "",
        "no-scheme",
        "relative/path",
        "https://a b/",
        "https://%zz/",
    ] {
        let contents = format!(r#"{{"name": "n", "version": "1", "website": "{website}"}}"#);
        let diagnostics = validate_manifest(&manifest(&contents));
        assert_single(
            &diagnostics,
            "project-website-iri",
            ".project.json",
            Severity::Error,
        );
    }
}

#[test]
fn invalid_usage_resource_is_reported_with_its_position() {
    let diagnostics = validate_manifest(&manifest(
        r#"{"name": "n", "version": "1", "usage": [
            {"resource": "urn:kpar:good"},
            {"resource": "not an IRI"}
        ]}"#,
    ));
    assert_single(
        &diagnostics,
        "project-usage-iri",
        ".project.json",
        Severity::Error,
    );
    assert!(diagnostics[0].message.contains("usage[1]"));
}

#[test]
fn absent_iri_properties_are_not_reported() {
    let manifest = manifest(r#"{"name": "n", "version": "1"}"#);
    assert_eq!(validate_manifest(&manifest), []);
}

#[test]
fn missing_metadata_is_a_warning() {
    let dir = project();
    assert_single(
        &validate_metadata(dir.path(), None),
        "meta-missing",
        ".meta.json",
        Severity::Warning,
    );
}

#[test]
fn invalid_created_is_reported() {
    let dir = project();
    let contents = r#"{"index": {"A": "A.sysml"}, "created": "2026-02-30T12:00:00Z"}"#;
    assert_single(
        &validate(dir.path(), contents),
        "meta-created-format",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn iso_8601_date_without_offset_is_reported() {
    let dir = project();
    let contents = r#"{"index": {"A": "A.sysml"}, "created": "2026-10-31T12:00:00"}"#;
    assert_eq!(
        rule_ids(&validate(dir.path(), contents)),
        ["meta-created-format"]
    );
}

#[test]
fn invalid_metamodel_is_reported() {
    let dir = project();
    let contents = format!(
        r#"{{"index": {{"A": "A.sysml"}}, "created": "{CREATED}", "metamodel": "not an IRI"}}"#
    );
    assert_single(
        &validate(dir.path(), &contents),
        "meta-metamodel-iri",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn metamodel_with_a_fragment_is_accepted() {
    // A fragment is allowed in every IRI property (docs/specs/project-metadata.md, section 4.1).
    let dir = project();
    let contents = format!(
        r#"{{"index": {{"A": "A.sysml"}}, "created": "{CREATED}", "metamodel": "https://www.omg.org/spec/SysML/20250201#model"}}"#
    );
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn index_paths_must_follow_the_path_convention() {
    for path in [
        "",
        "/A.sysml",
        "./A.sysml",
        "../A.sysml",
        "models/../A.sysml",
        "models//A.sysml",
        "models/",
        "models\\\\A.sysml",
        "C:/A.sysml",
        "C:A.sysml",
        "NoExtension",
        ".project.json",
        ".meta.json",
        "A\\u0000.sysml",
    ] {
        let dir = project();
        let contents = metadata_json(&format!(r#"{{"A": "A.sysml", "B": "{path}"}}"#), "{}");
        assert_single(
            &validate(dir.path(), &contents),
            "index-path-syntax",
            ".meta.json",
            Severity::Error,
        );
    }
}

#[test]
fn checksum_paths_must_follow_the_path_convention() {
    for path in ["/A.sysml", "../A.sysml", "models\\\\A.sysml"] {
        let dir = project();
        let contents = metadata_json_with_a_indexed(&sha256_checksum(path, A_SHA256));
        assert_single(
            &validate(dir.path(), &contents),
            "checksum-path-syntax",
            ".meta.json",
            Severity::Error,
        );
    }
}

#[test]
fn missing_index_file_is_reported() {
    let dir = project();
    let contents = metadata_json(r#"{"A": "A.sysml", "B": "models/B.sysml"}"#, "{}");
    assert_single(
        &validate(dir.path(), &contents),
        "index-file-missing",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn index_entry_must_be_a_regular_file() {
    let dir = project();
    fs::create_dir(dir.path().join("Directory.sysml")).unwrap();
    let contents = metadata_json(r#"{"A": "A.sysml", "D": "Directory.sysml"}"#, "{}");
    assert_eq!(
        rule_ids(&validate(dir.path(), &contents)),
        ["index-file-missing"]
    );
}

#[test]
fn index_path_through_a_file_is_reported() {
    let dir = project();
    let contents = metadata_json(r#"{"A": "A.sysml", "B": "A.sysml/B.sysml"}"#, "{}");
    assert_eq!(
        rule_ids(&validate(dir.path(), &contents)),
        ["index-file-missing"]
    );
}

#[test]
fn index_accepts_every_standard_model_file_format() {
    // KerML 1.0 section 10.2 allows `.json` and `.xmi`, which discovery doesn't list.
    let dir = project();
    write_file(&dir.path().join("structure/assembly/Body.json"), "{}");
    write_file(&dir.path().join("Model.xmi"), "");
    let contents = metadata_json(
        r#"{"A": "A.sysml", "Body": "structure/assembly/Body.json", "Model": "Model.xmi"}"#,
        "{}",
    );
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn several_names_can_refer_to_one_file() {
    let dir = project();
    let contents = metadata_json(r#"{"A": "A.sysml", "Alias": "A.sysml"}"#, "{}");
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn index_accepts_explicit_paths_into_hidden_directories() {
    let dir = project();
    write_file(&dir.path().join(".hidden/B.sysml"), "");
    let contents = metadata_json(r#"{"A": "A.sysml", "B": ".hidden/B.sysml"}"#, "{}");
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn index_path_into_a_nested_project_is_reported() {
    let dir = project();
    write_file(&dir.path().join("nested/.project.json"), "{}");
    write_file(&dir.path().join("nested/models/B.sysml"), "");
    let contents = metadata_json(r#"{"A": "A.sysml", "B": "nested/models/B.sysml"}"#, "{}");
    assert_single(
        &validate(dir.path(), &contents),
        "index-path-boundary",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn checksum_path_into_a_nested_project_is_reported() {
    let dir = project();
    write_file(&dir.path().join("nested/.project.json"), "{}");
    write_file(&dir.path().join("nested/B.sysml"), "");
    let contents = metadata_json_with_a_indexed(&sha256_checksum("nested/B.sysml", A_SHA256));
    assert_single(
        &validate(dir.path(), &contents),
        "checksum-path-boundary",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn unreferenced_model_file_is_a_warning_against_that_file() {
    let dir = project();
    write_file(&dir.path().join("models/B.kerml"), "");
    let contents = metadata_json(r#"{"A": "A.sysml"}"#, "{}");
    assert_single(
        &validate(dir.path(), &contents),
        "index-unreferenced-file",
        "models/B.kerml",
        Severity::Warning,
    );
}

#[test]
fn matching_checksum_produces_no_diagnostics() {
    let dir = project();
    let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", A_SHA256));
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn checksum_value_comparison_ignores_hex_case() {
    let dir = project();
    let upper = A_SHA256.to_uppercase();
    let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", &upper));
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn partial_checksums_are_allowed() {
    // Table 13: checksums cover "some or all" of the model interchange files.
    let dir = project();
    write_file(&dir.path().join("B.sysml"), "");
    let contents = metadata_json(
        r#"{"A": "A.sysml", "B": "B.sysml"}"#,
        &sha256_checksum("A.sysml", A_SHA256),
    );
    assert_eq!(validate(dir.path(), &contents), []);
}

#[test]
fn checksum_mismatch_is_an_error() {
    let dir = project();
    let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", A_SHA256));
    write_file(&dir.path().join("A.sysml"), "package Changed;\n");
    assert_single(
        &validate(dir.path(), &contents),
        "checksum-mismatch",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn changing_one_checksum_hex_digit_is_a_mismatch() {
    let dir = project();
    let mut value = A_SHA256.to_owned();
    value.replace_range(63.., if value.ends_with('0') { "1" } else { "0" });
    let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", &value));
    assert_eq!(
        rule_ids(&validate(dir.path(), &contents)),
        ["checksum-mismatch"]
    );
}

#[test]
fn malformed_sha256_value_is_not_reported_as_a_mismatch() {
    let too_short = &A_SHA256[..62];
    let too_long = format!("{A_SHA256}00");
    let not_hex = format!("{}zz", &A_SHA256[..62]);
    let non_ascii = format!("{}é", &A_SHA256[..62]);
    for value in [too_short, &too_long, &not_hex, &non_ascii, ""] {
        let dir = project();
        let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", value));
        assert_single(
            &validate(dir.path(), &contents),
            "checksum-value-invalid",
            ".meta.json",
            Severity::Error,
        );
    }
}

#[test]
fn missing_checksum_file_is_reported() {
    let dir = project();
    let contents = metadata_json_with_a_indexed(&sha256_checksum("Gone.sysml", A_SHA256));
    assert_single(
        &validate(dir.path(), &contents),
        "checksum-file-missing",
        ".meta.json",
        Severity::Error,
    );
}

#[test]
fn algorithm_outside_note_6_is_an_error() {
    // Spellings are exact: Note 6 has `SHA256` and `SHA-384`, not `SHA-256` or `SHA384`.
    for algorithm in ["SHA-256", "SHA384", "sha256", "SHA512", ""] {
        let dir = project();
        let checksum = format!(r#"{{"A.sysml": {{"value": "00", "algorithm": "{algorithm}"}}}}"#);
        assert_single(
            &validate(dir.path(), &metadata_json_with_a_indexed(&checksum)),
            "checksum-algorithm-invalid",
            ".meta.json",
            Severity::Error,
        );
    }
}

#[test]
fn valid_unsupported_algorithm_is_an_unverified_warning_not_a_pass() {
    for algorithm in [
        "SHA1",
        "SHA224",
        "SHA-384",
        "SHA3-256",
        "SHA3-384",
        "SHA3-512",
        "BLAKE2b-256",
        "BLAKE2b-384",
        "BLAKE2b-512",
        "BLAKE3",
        "MD2",
        "MD4",
        "MD5",
        "MD6",
        "ADLER32",
    ] {
        let dir = project();
        let checksum = format!(r#"{{"A.sysml": {{"value": "00", "algorithm": "{algorithm}"}}}}"#);
        assert_single(
            &validate(dir.path(), &metadata_json_with_a_indexed(&checksum)),
            "checksum-algorithm-unsupported",
            ".meta.json",
            Severity::Warning,
        );
    }
}

#[test]
fn unsupported_algorithm_on_a_missing_file_still_reports_the_file() {
    let dir = project();
    let checksum = r#"{"Gone.sysml": {"value": "00", "algorithm": "MD5"}}"#;
    assert_eq!(
        rule_ids(&validate(
            dir.path(),
            &metadata_json_with_a_indexed(checksum)
        )),
        ["checksum-file-missing", "checksum-algorithm-unsupported"]
    );
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::process::Command;

    #[test]
    fn symbolic_link_as_index_file_is_reported() {
        let outside = tempfile::tempdir().unwrap();
        write_file(&outside.path().join("Outside.sysml"), "");
        let dir = project();
        symlink(
            outside.path().join("Outside.sysml"),
            dir.path().join("Link.sysml"),
        )
        .unwrap();
        let contents = metadata_json(r#"{"A": "A.sysml", "L": "Link.sysml"}"#, "{}");
        assert_eq!(
            rule_ids(&validate(dir.path(), &contents)),
            ["index-path-boundary"]
        );
    }

    #[test]
    fn symbolic_link_directory_in_index_path_is_reported() {
        let outside = tempfile::tempdir().unwrap();
        write_file(&outside.path().join("Outside.sysml"), "");
        let dir = project();
        symlink(outside.path(), dir.path().join("linked")).unwrap();
        let contents = metadata_json(r#"{"A": "A.sysml", "L": "linked/Outside.sysml"}"#, "{}");
        assert_eq!(
            rule_ids(&validate(dir.path(), &contents)),
            ["index-path-boundary"]
        );
    }

    #[test]
    fn checksum_never_reads_through_a_symbolic_link() {
        let outside = tempfile::tempdir().unwrap();
        write_file(&outside.path().join("Outside.sysml"), "package A;\n");
        let dir = project();
        symlink(
            outside.path().join("Outside.sysml"),
            dir.path().join("Link.sysml"),
        )
        .unwrap();
        // The linked file has the recorded hash, but it isn't in the project.
        let contents = metadata_json_with_a_indexed(&sha256_checksum("Link.sysml", A_SHA256));
        assert_eq!(
            rule_ids(&validate(dir.path(), &contents)),
            ["checksum-path-boundary"]
        );
    }

    #[test]
    fn named_pipe_is_not_read() {
        // Opening a FIFO blocks until a writer appears, so this test hangs if it is read.
        let dir = project();
        let pipe = dir.path().join("Pipe.sysml");
        assert!(
            Command::new("mkfifo")
                .arg(&pipe)
                .status()
                .unwrap()
                .success()
        );
        let contents = metadata_json_with_a_indexed(&sha256_checksum("Pipe.sysml", A_SHA256));
        assert_eq!(
            rule_ids(&validate(dir.path(), &contents)),
            ["checksum-file-missing"]
        );
    }

    #[test]
    fn unreadable_checksum_file_is_an_error_not_a_pass() {
        let dir = project();
        let path = dir.path().join("A.sysml");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&path).is_ok() {
            return; // Running as root: permissions are not enforced.
        }
        let contents = metadata_json_with_a_indexed(&sha256_checksum("A.sysml", A_SHA256));
        assert_single(
            &validate(dir.path(), &contents),
            "file-unreadable",
            "A.sysml",
            Severity::Error,
        );
    }

    #[test]
    fn unreadable_directory_in_a_path_is_an_error_not_a_pass() {
        let dir = project();
        let locked = dir.path().join("locked");
        write_file(&locked.join("B.sysml"), "");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(&locked).is_ok();
        let contents = metadata_json(r#"{"A": "A.sysml", "B": "locked/B.sysml"}"#, "{}");
        let diagnostics = validate(dir.path(), &contents);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            return; // Running as root: permissions are not enforced.
        }
        assert_eq!(
            rule_ids(&diagnostics),
            ["file-unreadable", "file-unreadable"]
        );
        assert_eq!(diagnostics[0].path, "locked");
        assert_eq!(diagnostics[1].path, "locked/B.sysml");
    }

    #[test]
    fn traversal_continues_past_unreadable_directories() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join(".project.json"), "{}");
        write_file(&dir.path().join("Unindexed.sysml"), "");
        // Discovery skips hidden directories, so this one must not be reported.
        let locked = ["locked-a", "locked-b", ".locked-hidden"].map(|name| dir.path().join(name));
        for directory in &locked {
            write_file(&directory.join("Inaccessible.sysml"), "");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o000)).unwrap();
        }
        let readable = fs::read_dir(&locked[0]).is_ok();
        let first = validate(dir.path(), &metadata_json("{}", "{}"));
        let second = validate(dir.path(), &metadata_json("{}", "{}"));
        for directory in &locked {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        if readable {
            return; // Running as root: permissions are not enforced.
        }

        let summary: Vec<(&str, &str, Severity)> = first
            .iter()
            .map(|d| (d.path.as_str(), d.rule.id(), d.severity))
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "Unindexed.sysml",
                    "index-unreferenced-file",
                    Severity::Warning
                ),
                ("locked-a", "file-unreadable", Severity::Error),
                ("locked-b", "file-unreadable", Severity::Error),
            ]
        );
        assert_eq!(first, second);
    }

    #[test]
    fn unreadable_project_root_is_reported_against_the_root() {
        let dir = project();
        let path = dir.path().join(".meta.json");
        fs::write(&path, metadata_json("{}", "{}")).unwrap();
        let parsed = read_metadata(&path).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(dir.path()).is_ok();
        let diagnostics = validate_metadata(dir.path(), Some(&parsed));
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            return; // Running as root: permissions are not enforced.
        }
        assert_single(&diagnostics, "file-unreadable", ".", Severity::Error);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn traversal_continues_past_a_non_utf8_file_name() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join(".project.json"), "{}");
        write_file(&dir.path().join("Unindexed.sysml"), "");
        fs::write(dir.path().join(OsStr::from_bytes(b"Bad\xFF.sysml")), "").unwrap();

        let diagnostics = validate(dir.path(), &metadata_json("{}", "{}"));
        assert_eq!(
            rule_ids(&diagnostics),
            ["file-unreadable", "index-unreferenced-file"]
        );
        assert_eq!(diagnostics[1].path, "Unindexed.sysml");
    }

    #[test]
    fn unlistable_directory_is_an_error_not_a_pass() {
        let dir = project();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(&locked).is_ok();
        let diagnostics = validate(dir.path(), &metadata_json(r#"{"A": "A.sysml"}"#, "{}"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            return; // Running as root: permissions are not enforced.
        }
        assert_eq!(rule_ids(&diagnostics), ["file-unreadable"]);
    }
}
