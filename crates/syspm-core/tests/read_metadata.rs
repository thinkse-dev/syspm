mod common;

use common::fixture;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use syspm_core::{ProjectMetadata, ReadMetadataError, read_metadata};

/// Writes `contents` to a `.meta.json` in a new temporary directory and reads it. The directory
/// is removed when `dir` goes out of scope at the end of this function.
fn read_temp_metadata(contents: &str) -> (PathBuf, Result<ProjectMetadata, ReadMetadataError>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".meta.json");
    std::fs::write(&path, contents).unwrap();

    let result = read_metadata(&path);
    (path, result)
}

/// Asserts that `result` failed with `ReadMetadataError::Parse`, that the message names `path`,
/// and that the JSON error is in the source rather than repeated in the message.
fn assert_parse_error_naming(path: &Path, result: Result<ProjectMetadata, ReadMetadataError>) {
    let error = result.unwrap_err();
    assert!(matches!(error, ReadMetadataError::Parse { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));

    let source = std::error::Error::source(&error).expect("parse error has a source");
    assert!(source.downcast_ref::<serde_json::Error>().is_some());
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn reads_every_table_13_property() {
    let metadata = read_metadata(&fixture("all-properties/.meta.json")).unwrap();
    assert_eq!(metadata.index.len(), 2);
    assert_eq!(metadata.index["AllProperties"], "AllProperties.sysml");
    assert_eq!(metadata.index["AllPropertiesExtras"], "Extras.sysml");
    assert_eq!(metadata.created, "2026-10-30T12:00:00Z");
    assert_eq!(
        metadata.metamodel.as_deref(),
        Some("https://www.omg.org/spec/SysML/20250201")
    );
    assert_eq!(metadata.includes_derived, Some(false));
    assert_eq!(metadata.includes_implied, Some(true));

    assert_eq!(metadata.checksum.len(), 2);
    let checksum = &metadata.checksum["AllProperties.sysml"];
    assert_eq!(
        checksum.value,
        "8713525aca93f849344d0e802f44805f3fd56b1c43ca2db03512c17535e8f76f"
    );
    assert_eq!(checksum.algorithm, "SHA256");
}

#[test]
fn maps_iterate_in_sorted_order_regardless_of_file_order() {
    // The fixture lists both maps out of order, and iteration must not depend on that.
    let metadata = read_metadata(&fixture("all-properties/.meta.json")).unwrap();
    let names: Vec<&str> = metadata.index.keys().map(String::as_str).collect();
    assert_eq!(names, ["AllProperties", "AllPropertiesExtras"]);
    let paths: Vec<&str> = metadata.checksum.keys().map(String::as_str).collect();
    assert_eq!(paths, ["AllProperties.sysml", "Extras.sysml"]);
}

#[test]
fn reads_required_properties_only() {
    let metadata = read_metadata(&fixture("common-interfaces/.meta.json")).unwrap();
    assert_eq!(metadata.index["CommonInterfaces"], "CommonInterfaces.sysml");
    assert_eq!(metadata.created, "2026-10-30T12:00:00Z");
    assert_eq!(metadata.metamodel, None);
    assert_eq!(metadata.includes_derived, None);
    assert_eq!(metadata.includes_implied, None);
    assert!(metadata.checksum.is_empty());
}

#[test]
fn reads_empty_index() {
    // The schema requires `index` but allows it to be empty.
    let (_, result) = read_temp_metadata(r#"{"index": {}, "created": "2026-10-30T12:00:00Z"}"#);
    assert!(result.unwrap().index.is_empty());
}

#[test]
fn unknown_properties_are_kept() {
    let (_, result) = read_temp_metadata(
        r#"{
            "index": {},
            "created": "2026-10-30T12:00:00Z",
            "x-tool-setting": [1, 2, 3],
            "checksum": {"A.sysml": {"value": "00", "algorithm": "SHA256", "extra": true}}
        }"#,
    );
    let metadata = result.unwrap();
    assert_eq!(metadata.checksum["A.sysml"].value, "00");
    assert_eq!(
        Vec::from_iter(&metadata.additional_properties),
        [(&"x-tool-setting".to_string(), &json!([1, 2, 3]))]
    );
    assert_eq!(
        Vec::from_iter(&metadata.checksum["A.sysml"].additional_properties),
        [(&"extra".to_string(), &json!(true))]
    );
}

/// See `reads_manifest_written_by_another_tool` in `read_manifest.rs` for where this comes from.
#[test]
fn reads_metadata_written_by_another_tool() {
    let metadata = read_metadata(&fixture("toolkit-archive/.meta.json")).unwrap();
    assert_eq!(Vec::from_iter(metadata.index.keys()), ["UnitsLib"]);
    assert_eq!(metadata.index["UnitsLib"], "UnitsLib.sysml");
    assert_eq!(
        metadata.metamodel.as_deref(),
        Some("https://www.omg.org/spec/SysML/20250201")
    );
    assert_eq!(metadata.checksum["UnitsLib.sysml"].algorithm, "SHA256");
    assert!(metadata.additional_properties.is_empty());
}

#[test]
fn reads_fixture_additional_properties() {
    let metadata = read_metadata(&fixture("additional-properties/.meta.json")).unwrap();
    assert_eq!(
        metadata.additional_properties["x-example-generator"],
        json!("fixture")
    );
    assert_eq!(
        metadata.checksum["Extended.sysml"].additional_properties["x-example-scope"],
        json!("raw bytes")
    );
}

#[test]
fn defined_properties_are_not_additional() {
    let metadata = read_metadata(&fixture("all-properties/.meta.json")).unwrap();
    assert!(metadata.additional_properties.is_empty());
    assert!(
        metadata
            .checksum
            .values()
            .all(|checksum| checksum.additional_properties.is_empty())
    );
}

#[test]
fn missing_metadata_is_io_error_naming_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".meta.json");

    let error = read_metadata(&path).unwrap_err();
    assert!(matches!(error, ReadMetadataError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));

    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(source.downcast_ref::<std::io::Error>().is_some());
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn missing_index_is_parse_error_naming_path() {
    let (path, result) = read_temp_metadata(r#"{"created": "2026-10-30T12:00:00Z"}"#);
    assert_parse_error_naming(&path, result);
}

#[test]
fn missing_created_is_parse_error_naming_path() {
    let (path, result) = read_temp_metadata(r#"{"index": {}}"#);
    assert_parse_error_naming(&path, result);
}

#[test]
fn malformed_json_is_parse_error() {
    let (path, result) = read_temp_metadata(r#"{"index": {}, "created": "#);
    assert_parse_error_naming(&path, result);
}

#[test]
fn wrong_property_type_is_parse_error() {
    let cases = [
        // An index entry maps a name to a path string.
        r#""index": {"A": 1}, "created": "2026-10-30T12:00:00Z""#,
        r#""index": [], "created": "2026-10-30T12:00:00Z""#,
        r#""index": {}, "created": 20261030"#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "includesDerived": "false""#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "metamodel": 1"#,
        // The specification text describes a list of checksums per file, but the normative
        // schema defines a single object, which is what syspm accepts.
        r#""index": {}, "created": "2026-10-30T12:00:00Z",
            "checksum": {"A.sysml": [{"value": "00", "algorithm": "SHA256"}]}"#,
        // Both `value` and `algorithm` are required in a checksum.
        r#""index": {}, "created": "2026-10-30T12:00:00Z",
            "checksum": {"A.sysml": {"value": "00"}}"#,
    ];
    for properties in cases {
        let (path, result) = read_temp_metadata(&format!("{{{properties}}}"));
        assert_parse_error_naming(&path, result);
    }
}

#[test]
fn metadata_as_array_is_parse_error() {
    // Fields must not be matched by position: the schema requires an object.
    let (path, result) = read_temp_metadata(r#"[{}, "x", "iri", false, true, {}]"#);
    assert_parse_error_naming(&path, result);
}

#[test]
fn checksum_as_array_is_parse_error() {
    // Fields must not be matched by position: the schema requires an object.
    let (path, result) = read_temp_metadata(
        r#"{"index": {}, "created": "x", "checksum": {"A.sysml": ["00", "SHA256"]}}"#,
    );
    assert_parse_error_naming(&path, result);
}

#[test]
fn duplicate_keys_are_parse_errors() {
    let cases = [
        r#"{"index": {"A": "A.sysml", "A": "B.sysml"}, "created": "x"}"#,
        // Escapes are decoded first, so these two keys are the same key.
        r#"{"index": {"A": "A.sysml", "\u0041": "B.sysml"}, "created": "x"}"#,
        r#"{"index": {}, "created": "x", "created": "y"}"#,
        r#"{"index": {}, "created": "x", "checksum": {
            "A.sysml": {"value": "00", "algorithm": "SHA256"},
            "A.sysml": {"value": "11", "algorithm": "SHA256"}
        }}"#,
        r#"{"index": {}, "created": "x", "checksum": {
            "A.sysml": {"value": "00", "value": "11", "algorithm": "SHA256"}
        }}"#,
        // Properties the schema doesn't define are kept, so they can't repeat either.
        r#"{"index": {}, "created": "x", "x": 1, "x": 2}"#,
        r#"{"index": {}, "created": "x", "checksum": {
            "A.sysml": {"value": "00", "algorithm": "SHA256", "x": 1, "x": 2}
        }}"#,
        // At any depth.
        r#"{"index": {}, "created": "x", "y": {"x": 1, "x": 2}}"#,
        r#"{"index": {}, "created": "x", "checksum": {
            "A.sysml": {"value": "00", "algorithm": "SHA256", "y": [{"x": 1, "x": 2}]}
        }}"#,
    ];
    for contents in cases {
        let (path, result) = read_temp_metadata(contents);
        assert_parse_error_naming(&path, result);
    }
}

#[test]
fn duplicate_key_error_names_the_key() {
    let (_, result) = read_temp_metadata(r#"{"index": {"A": "a", "A": "b"}, "created": "x"}"#);
    let error = result.unwrap_err();
    let source = std::error::Error::source(&error).expect("parse error has a source");
    assert!(source.to_string().contains("duplicate key `A`"), "{source}");
}

#[test]
fn null_property_is_parse_error() {
    let cases = [
        r#""index": null, "created": "2026-10-30T12:00:00Z""#,
        r#""index": {}, "created": null"#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "metamodel": null"#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "includesDerived": null"#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "includesImplied": null"#,
        r#""index": {}, "created": "2026-10-30T12:00:00Z", "checksum": null"#,
    ];
    for properties in cases {
        let (path, result) = read_temp_metadata(&format!("{{{properties}}}"));
        assert_parse_error_naming(&path, result);
    }
}

#[cfg(unix)]
#[test]
fn valid_metadata_symlink_is_io_error_naming_path() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    fs::write(
        outside.path().join(".meta.json"),
        r#"{"index": {}, "created": "2026-10-31T12:00:00Z"}"#,
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    symlink(
        outside.path().join(".meta.json"),
        dir.path().join(".meta.json"),
    )
    .unwrap();

    let error = read_metadata(&dir.path().join(".meta.json")).unwrap_err();
    assert!(matches!(error, ReadMetadataError::Io { .. }));
    assert!(
        error
            .to_string()
            .contains(&dir.path().display().to_string())
    );

    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(source.downcast_ref::<std::io::Error>().is_some());
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn directory_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".meta.json");
    fs::create_dir(&path).unwrap();

    let error = read_metadata(&path).unwrap_err();
    assert!(matches!(error, ReadMetadataError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));
}
