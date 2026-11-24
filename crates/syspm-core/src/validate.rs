//! Checks parsed project files against KerML 1.0 sections 10.2 and 10.3, and against the files
//! on disk.
//!
//! The rules and their identifiers are in docs/specs/project-metadata.md, section 4. Validation
//! collects every problem instead of stopping at the first. A problem with the project is a
//! [`Diagnostic`], not an `Err`. The functions here always return, so one bad file can't hide
//! findings about another.
//!
//! Path checks inspect the file system and then use it, so they aren't race-resistant. Another
//! process that changes the project during validation can invalidate a check that passed. They
//! protect against what a project contains, not against concurrent changes.

use crate::discover::{
    DiscoverError, ModelFileScan, is_project_root, scan_model_files, to_slash_path,
};
use crate::interchange::{PROJECT_MANIFEST, PROJECT_METADATA};
use crate::{ProjectManifest, ProjectMetadata};
use iri_string::types::IriStr;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The checksum algorithms KerML 1.0 Table 13 note 6 allows, spelled exactly as it spells them.
///
/// These are the identifiers the standard permits, not the algorithms syspm can compute. Only
/// SHA-256 is actually verified.
const STANDARD_CHECKSUM_ALGORITHMS: [&str; 16] = [
    "SHA1",
    "SHA224",
    "SHA256",
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
];

/// The length of a SHA-256 digest in bytes. Its hexadecimal form is twice as long.
const SHA256_DIGEST_BYTES: usize = 32;
const HASH_BUFFER_SIZE: usize = 64 * 1024;

/// How serious a [`Diagnostic`] is. A warning doesn't make a project invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    /// The severity's stable identifier: `warning` or `error`.
    pub fn id(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

/// The rule a [`Diagnostic`] reports. [`Rule::id`] gives its stable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// `website` in `.project.json` is not an IRI with a scheme.
    ProjectWebsiteIri,
    /// A `usage` `resource` in `.project.json` is not an IRI with a scheme.
    ProjectUsageIri,
    /// The project has no `.meta.json`.
    MetaMissing,
    /// `created` is not an RFC 3339 date-time.
    MetaCreatedFormat,
    /// `metamodel` is not an IRI with a scheme.
    MetaMetamodelIri,
    /// An `index` path does not follow Table 13 note 2.
    IndexPathSyntax,
    /// An `index` path leaves the project through a nested project or a symbolic link.
    IndexPathBoundary,
    /// An `index` path does not refer to a regular file.
    IndexFileMissing,
    /// A model file found by discovery (see [`crate::list_model_files`]) is not referred to by
    /// `index`.
    IndexUnreferencedFile,
    /// A `checksum` path does not follow Table 13 note 2.
    ChecksumPathSyntax,
    /// A `checksum` path leaves the project through a nested project or a symbolic link.
    ChecksumPathBoundary,
    /// A `checksum` path does not refer to a regular file.
    ChecksumFileMissing,
    /// A checksum algorithm is not one that Table 13 note 6 allows.
    ChecksumAlgorithmInvalid,
    /// A checksum uses a valid algorithm that syspm can't compute, so the file was **not**
    /// verified. Having no error doesn't mean the file was verified. This rule keeps the two
    /// apart, for policies that require verification.
    ChecksumAlgorithmUnsupported,
    /// A checksum value is not a valid representation for its algorithm.
    ChecksumValueInvalid,
    /// A file's contents do not match its recorded checksum.
    ChecksumMismatch,
    /// A file or directory that had to be read couldn't be. This covers reading a file to verify
    /// its checksum, listing a directory during discovery, inspecting a recorded path, and finding
    /// a model file whose name isn't valid UTF-8.
    FileUnreadable,
}

impl Rule {
    /// Every rule, in declaration order.
    pub const ALL: [Rule; 17] = [
        Rule::ProjectWebsiteIri,
        Rule::ProjectUsageIri,
        Rule::MetaMissing,
        Rule::MetaCreatedFormat,
        Rule::MetaMetamodelIri,
        Rule::IndexPathSyntax,
        Rule::IndexPathBoundary,
        Rule::IndexFileMissing,
        Rule::IndexUnreferencedFile,
        Rule::ChecksumPathSyntax,
        Rule::ChecksumPathBoundary,
        Rule::ChecksumFileMissing,
        Rule::ChecksumAlgorithmInvalid,
        Rule::ChecksumAlgorithmUnsupported,
        Rule::ChecksumValueInvalid,
        Rule::ChecksumMismatch,
        Rule::FileUnreadable,
    ];

    /// The rule's stable identifier, such as `checksum-mismatch`.
    pub fn id(self) -> &'static str {
        match self {
            Rule::ProjectWebsiteIri => "project-website-iri",
            Rule::ProjectUsageIri => "project-usage-iri",
            Rule::MetaMissing => "meta-missing",
            Rule::MetaCreatedFormat => "meta-created-format",
            Rule::MetaMetamodelIri => "meta-metamodel-iri",
            Rule::IndexPathSyntax => "index-path-syntax",
            Rule::IndexPathBoundary => "index-path-boundary",
            Rule::IndexFileMissing => "index-file-missing",
            Rule::IndexUnreferencedFile => "index-unreferenced-file",
            Rule::ChecksumPathSyntax => "checksum-path-syntax",
            Rule::ChecksumPathBoundary => "checksum-path-boundary",
            Rule::ChecksumFileMissing => "checksum-file-missing",
            Rule::ChecksumAlgorithmInvalid => "checksum-algorithm-invalid",
            Rule::ChecksumAlgorithmUnsupported => "checksum-algorithm-unsupported",
            Rule::ChecksumValueInvalid => "checksum-value-invalid",
            Rule::ChecksumMismatch => "checksum-mismatch",
            Rule::FileUnreadable => "file-unreadable",
        }
    }

    /// How serious a violation of this rule is.
    pub fn severity(self) -> Severity {
        match self {
            Rule::MetaMissing
            | Rule::IndexUnreferencedFile
            | Rule::ChecksumAlgorithmUnsupported => Severity::Warning,
            Rule::ProjectWebsiteIri
            | Rule::ProjectUsageIri
            | Rule::MetaCreatedFormat
            | Rule::MetaMetamodelIri
            | Rule::IndexPathSyntax
            | Rule::IndexPathBoundary
            | Rule::IndexFileMissing
            | Rule::ChecksumPathSyntax
            | Rule::ChecksumPathBoundary
            | Rule::ChecksumFileMissing
            | Rule::ChecksumAlgorithmInvalid
            | Rule::ChecksumValueInvalid
            | Rule::ChecksumMismatch
            | Rule::FileUnreadable => Severity::Error,
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

/// One problem found in a project.
///
/// Diagnostics order by `path`, then `rule`, so sorted output is the same on every run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    /// Where the problem is: a file or directory relative to the project root, with `/`
    /// separators, or `.` for the root itself.
    pub path: String,
    pub rule: Rule,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    fn new(path: &str, rule: Rule, message: String) -> Self {
        Diagnostic {
            path: path.to_owned(),
            rule,
            severity: rule.severity(),
            message,
        }
    }
}

/// Checks what the types of [`ProjectManifest`] can't express, and returns sorted diagnostics.
pub fn validate_manifest(manifest: &ProjectManifest) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut report = |rule, message| {
        diagnostics.push(Diagnostic::new(PROJECT_MANIFEST, rule, message));
    };

    if let Some(website) = manifest
        .website
        .as_deref()
        .filter(|iri| !is_absolute_iri(iri))
    {
        report(
            Rule::ProjectWebsiteIri,
            format!("website is not an IRI with a scheme: `{website}`"),
        );
    }
    for (position, usage) in manifest.usage.iter().enumerate() {
        if !is_absolute_iri(&usage.resource) {
            let resource = &usage.resource;
            report(
                Rule::ProjectUsageIri,
                format!("usage[{position}] resource is not an IRI with a scheme: `{resource}`"),
            );
        }
    }

    diagnostics.sort();
    diagnostics
}

/// Checks the project in `root` against its metadata, and returns sorted diagnostics.
///
/// `metadata` is `None` when the project has no `.meta.json`. A `.meta.json` that can't be read
/// or parsed is an error from [`crate::read_metadata`], not a diagnostic.
pub fn validate_metadata(root: &Path, metadata: Option<&ProjectMetadata>) -> Vec<Diagnostic> {
    let Some(metadata) = metadata else {
        return vec![missing_metadata_diagnostic()];
    };
    let scan = scan_model_files(root);
    let mut diagnostics = validate_metadata_with_scan(root, metadata, &scan);
    diagnostics.extend(listing_diagnostics(root, &scan));
    diagnostics.sort();
    diagnostics
}

/// Checks the project in `root` against its metadata, using a `scan` the caller already has, and
/// returns sorted diagnostics.
///
/// Unlike [`validate_metadata`], this doesn't report `scan.failures`, since they don't depend on
/// the metadata. Callers get those from [`listing_diagnostics`]. It also needs parsed metadata, so
/// it doesn't report a missing `.meta.json` either. That's [`missing_metadata_diagnostic`].
pub(crate) fn validate_metadata_with_scan(
    root: &Path,
    metadata: &ProjectMetadata,
    scan: &ModelFileScan,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    check_properties(metadata, &mut diagnostics);
    check_index(root, metadata, scan, &mut diagnostics);
    check_checksums(root, metadata, &mut diagnostics);
    diagnostics.sort();
    diagnostics
}

/// The warning for a project that has no `.meta.json`.
pub(crate) fn missing_metadata_diagnostic() -> Diagnostic {
    metadata_diagnostic(Rule::MetaMissing, "project has no metadata file".to_owned())
}

/// Reports each part of the project that `scan` couldn't list.
pub(crate) fn listing_diagnostics(root: &Path, scan: &ModelFileScan) -> Vec<Diagnostic> {
    scan.failures
        .iter()
        .map(|failure| listing_failure(root, failure))
        .collect()
}

/// Reports a part of the project that couldn't be listed. Unreferenced files are only reported
/// for what was reached, so this diagnostic is the sign that the result is incomplete.
fn listing_failure(root: &Path, error: &DiscoverError) -> Diagnostic {
    let (path, cause) = match error {
        DiscoverError::Io { path, source } => (Some(path), source.to_string()),
        DiscoverError::NonUtf8Path { path } => (Some(path), "name is not UTF-8".to_owned()),
        DiscoverError::ProjectNotFound { .. } => (None, error.to_string()),
    };
    let diagnostic_path = path
        .and_then(|path| path.strip_prefix(root).ok())
        .filter(|path| !path.as_os_str().is_empty())
        .map_or_else(
            || ".".to_owned(),
            |path| to_slash_path(path).unwrap_or_else(|| path.display().to_string()),
        );
    Diagnostic::new(
        &diagnostic_path,
        Rule::FileUnreadable,
        format!("cannot list model files: {cause}"),
    )
}

fn metadata_diagnostic(rule: Rule, message: String) -> Diagnostic {
    Diagnostic::new(PROJECT_METADATA, rule, message)
}

/// Whether `value` is an RFC 3987 IRI with a scheme. A fragment is allowed (see
/// docs/specs/project-metadata.md, section 4.1). `website`, `resource` and `metamodel` must all
/// pass.
fn is_absolute_iri(value: &str) -> bool {
    IriStr::new(value).is_ok()
}

/// Whether `value` is an RFC 3339 `date-time`, including a real calendar date.
///
/// The parser also accepts a space in place of `T`, which the RFC's grammar doesn't, so that is
/// rejected first. Every valid value has its separator in the same position.
fn is_rfc3339(value: &str) -> bool {
    matches!(value.as_bytes().get(10), Some(b'T' | b't'))
        && OffsetDateTime::parse(value, &Rfc3339).is_ok()
}

fn check_properties(metadata: &ProjectMetadata, diagnostics: &mut Vec<Diagnostic>) {
    if !is_rfc3339(&metadata.created) {
        let created = &metadata.created;
        diagnostics.push(metadata_diagnostic(
            Rule::MetaCreatedFormat,
            format!("created is not an RFC 3339 date-time: `{created}`"),
        ));
    }
    if let Some(metamodel) = metadata
        .metamodel
        .as_deref()
        .filter(|iri| !is_absolute_iri(iri))
    {
        diagnostics.push(metadata_diagnostic(
            Rule::MetaMetamodelIri,
            format!("metamodel is not an IRI with a scheme: `{metamodel}`"),
        ));
    }
}

fn check_index(
    root: &Path,
    metadata: &ProjectMetadata,
    scan: &ModelFileScan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, recorded_path) in &metadata.index {
        let entry_label = format!("index entry `{name}`");
        check_path(
            root,
            recorded_path,
            &entry_label,
            &INDEX_PATH_RULES,
            diagnostics,
        );
    }

    let referenced: BTreeSet<&str> = metadata.index.values().map(String::as_str).collect();
    for file in scan
        .files
        .iter()
        .filter(|f| !referenced.contains(f.as_str()))
    {
        diagnostics.push(Diagnostic::new(
            file,
            Rule::IndexUnreferencedFile,
            "model file is not referred to by the index".to_owned(),
        ));
    }
}

fn check_checksums(root: &Path, metadata: &ProjectMetadata, diagnostics: &mut Vec<Diagnostic>) {
    for (recorded_path, checksum) in &metadata.checksum {
        let entry_label = format!("checksum entry `{recorded_path}`");
        let disk_path = check_path(
            root,
            recorded_path,
            &entry_label,
            &CHECKSUM_PATH_RULES,
            diagnostics,
        );

        let algorithm = &checksum.algorithm;
        if !STANDARD_CHECKSUM_ALGORITHMS.contains(&algorithm.as_str()) {
            diagnostics.push(metadata_diagnostic(
                Rule::ChecksumAlgorithmInvalid,
                format!("{entry_label} has an invalid algorithm: `{algorithm}`"),
            ));
            continue;
        }
        if algorithm != "SHA256" {
            diagnostics.push(metadata_diagnostic(
                Rule::ChecksumAlgorithmUnsupported,
                format!("{entry_label} was not verified: algorithm `{algorithm}` is not supported"),
            ));
            continue;
        }
        let Some(expected) = decode_sha256(&checksum.value) else {
            diagnostics.push(metadata_diagnostic(
                Rule::ChecksumValueInvalid,
                format!("{entry_label} value is not 64 hexadecimal digits"),
            ));
            continue;
        };
        if let Some(disk_path) = disk_path {
            check_sha256(
                &disk_path,
                recorded_path,
                &entry_label,
                &expected,
                diagnostics,
            );
        }
    }
}

/// Compares the file at `disk_path` with its expected digest. Findings name `recorded_path` (the
/// path as `.meta.json` records it) or `entry_label`, which describes the entry.
fn check_sha256(
    disk_path: &Path,
    recorded_path: &str,
    entry_label: &str,
    expected: &[u8; SHA256_DIGEST_BYTES],
    diagnostics: &mut Vec<Diagnostic>,
) {
    match sha256_of_file(disk_path) {
        Ok(actual) if actual == *expected => {}
        Ok(_) => diagnostics.push(metadata_diagnostic(
            Rule::ChecksumMismatch,
            format!("{entry_label} does not match the file"),
        )),
        Err(error) => diagnostics.push(Diagnostic::new(
            recorded_path,
            Rule::FileUnreadable,
            format!("cannot read file: {error}"),
        )),
    }
}

/// Decodes a hexadecimal SHA-256 value in either case, so the comparison ignores case.
fn decode_sha256(value: &str) -> Option<[u8; SHA256_DIGEST_BYTES]> {
    let digits: Vec<u32> = value
        .chars()
        .map(|digit| digit.to_digit(16))
        .collect::<Option<_>>()?;
    if digits.len() != SHA256_DIGEST_BYTES * 2 {
        return None;
    }
    let mut bytes = [0; SHA256_DIGEST_BYTES];
    let (pairs, _) = digits.as_chunks::<2>();
    for (byte, [high, low]) in bytes.iter_mut().zip(pairs) {
        *byte = u8::try_from(high * 16 + low).ok()?;
    }
    Some(bytes)
}

/// Hashes a file's raw bytes without loading the whole file into memory.
fn sha256_of_file(path: &Path) -> io::Result<[u8; SHA256_DIGEST_BYTES]> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; HASH_BUFFER_SIZE];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok(hasher.finalize().into()),
            Ok(length) => hasher.update(&buffer[..length]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// The rules that path problems are reported under. They depend on where the path is recorded.
struct PathRules {
    syntax: Rule,
    boundary: Rule,
    missing: Rule,
}

const INDEX_PATH_RULES: PathRules = PathRules {
    syntax: Rule::IndexPathSyntax,
    boundary: Rule::IndexPathBoundary,
    missing: Rule::IndexFileMissing,
};

const CHECKSUM_PATH_RULES: PathRules = PathRules {
    syntax: Rule::ChecksumPathSyntax,
    boundary: Rule::ChecksumPathBoundary,
    missing: Rule::ChecksumFileMissing,
};

/// Reports any problem with a recorded path. Returns the file's location on disk only if it is
/// safe to read. This is the only way the checksum code gets a path to open.
fn check_path(
    root: &Path,
    recorded_path: &str,
    entry_label: &str,
    rules: &PathRules,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PathBuf> {
    if let Err(problem) = check_path_syntax(recorded_path) {
        diagnostics.push(metadata_diagnostic(
            rules.syntax,
            format!("{entry_label} has an invalid path `{recorded_path}`: {problem}"),
        ));
        return None;
    }

    match locate_recorded_path(root, recorded_path) {
        PathLookup::File(disk_path) => return Some(disk_path),
        PathLookup::Boundary(reason) => diagnostics.push(metadata_diagnostic(
            rules.boundary,
            format!("{entry_label} path `{recorded_path}` {reason}"),
        )),
        PathLookup::Missing => diagnostics.push(metadata_diagnostic(
            rules.missing,
            format!("{entry_label} path `{recorded_path}` is not a file in the project"),
        )),
        PathLookup::Unreadable(error) => diagnostics.push(Diagnostic::new(
            recorded_path,
            Rule::FileUnreadable,
            format!("cannot inspect path: {error}"),
        )),
    }
    None
}

/// Checks a path against Table 13 note 2, and rejects paths that would mean something else on
/// another platform. The note requires a path relative to the root, separated by `/`, and ending
/// in a file name with an extension. The two project files aren't model interchange files.
fn check_path_syntax(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("it is empty");
    }
    if path.starts_with('/') {
        return Err("it is absolute");
    }
    if path.contains('\\') {
        return Err("it contains a backslash");
    }
    if path.contains('\0') {
        return Err("it contains a NUL character");
    }
    if has_drive_prefix(path) {
        return Err("it has a drive prefix");
    }
    if path == PROJECT_MANIFEST || path == PROJECT_METADATA {
        return Err("it is a project file, not a model interchange file");
    }
    if path
        .split('/')
        .any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err("it contains an empty, `.`, or `..` segment");
    }
    let has_extension = Path::new(path)
        .extension()
        .is_some_and(|extension| !extension.is_empty());
    if !has_extension {
        return Err("it does not end in a file name with an extension");
    }
    Ok(())
}

fn has_drive_prefix(path: &str) -> bool {
    matches!(path.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic())
}

/// What a syntactically valid path refers to in the project.
enum PathLookup {
    File(PathBuf),
    Boundary(&'static str),
    Missing,
    Unreadable(io::Error),
}

/// Walks `path` one segment at a time, never following a symbolic link.
///
/// Checking each segment is what stops a link from leading out of the project, and requiring a
/// regular file is what stops a named pipe or device from being opened. Hidden directories are
/// allowed, because an explicit reference is deliberate, unlike what discovery finds by itself.
fn locate_recorded_path(root: &Path, path: &str) -> PathLookup {
    let mut current = root.to_path_buf();
    let mut segments = path.split('/').peekable();
    while let Some(segment) = segments.next() {
        current.push(segment);
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return PathLookup::Missing,
            Err(error) => return PathLookup::Unreadable(error),
        };
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return PathLookup::Boundary("passes through a symbolic link");
        }
        if segments.peek().is_none() {
            return if file_type.is_file() {
                PathLookup::File(current)
            } else {
                PathLookup::Missing
            };
        }
        if !file_type.is_dir() {
            return PathLookup::Missing;
        }
        if is_project_root(&current) {
            return PathLookup::Boundary("enters a nested project");
        }
    }
    PathLookup::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_rfc3339_date_times() {
        for (value, valid) in [
            ("2026-10-31T12:00:00Z", true),
            ("2026-10-31t12:00:00z", true),
            ("2026-10-31T12:00:00.5+05:30", true),
            ("2024-02-29T00:00:00Z", true),
            ("2016-12-31T23:59:60Z", true),
            ("2026-10-31 12:00:00Z", false),
            ("2026-02-29T00:00:00Z", false),
            ("2016-12-30T23:59:60Z", false),
            ("2026-10-31T12:00:00", false),
            ("20261031T120000Z", false),
            ("", false),
        ] {
            assert_eq!(is_rfc3339(value), valid, "{value}");
        }
    }

    #[test]
    fn decodes_sha256_of_either_case_to_the_same_bytes() {
        let lower = "00ff".repeat(16);
        let bytes = decode_sha256(&lower).unwrap();
        assert_eq!(bytes[..2], [0x00, 0xff]);
        assert_eq!(decode_sha256(&lower.to_uppercase()), Some(bytes));
    }

    #[test]
    fn rejects_sha256_values_with_a_sign_or_wrong_length() {
        assert_eq!(decode_sha256(&format!("+{}", "0".repeat(63))), None);
        assert_eq!(decode_sha256(&"0".repeat(63)), None);
        assert_eq!(decode_sha256(&"0".repeat(65)), None);
    }

    #[test]
    fn severities_display_their_identifiers() {
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
    }

    #[test]
    fn rule_identifiers_are_unique() {
        let ids: BTreeSet<&str> = Rule::ALL.iter().map(|rule| rule.id()).collect();
        assert_eq!(ids.len(), Rule::ALL.len());
    }
}
