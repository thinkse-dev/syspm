//! Minimal KerML project metadata reader.
//!
//! This is deliberately not a KPAR reader, dependency resolver, or OMG-schema validator.
//! Those responsibilities will be implemented after interoperability tests.

use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub usage: Vec<ProjectUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectUsage {
    pub resource: String,
    #[serde(default, rename = "versionConstraint")]
    pub version_constraint: Option<String>,
}

/// Failure to read a project manifest from disk.
///
/// The message names the manifest; the underlying cause is available through
/// [`std::error::Error::source`] rather than repeated in the message.
#[derive(Debug, thiserror::Error)]
pub enum ReadProjectError {
    #[error("cannot read project manifest {}", path.display())]
    Io { path: PathBuf, source: io::Error },
    #[error("invalid project manifest {}", path.display())]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// Read a project manifest. Full validation against the OMG JSON schema is deferred.
pub fn read_project(path: &Path) -> Result<ProjectManifest, ReadProjectError> {
    let contents = fs::read_to_string(path).map_err(|source| ReadProjectError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| ReadProjectError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_project_usages() {
        let raw = r#"{
            "name": "controller",
            "version": "0.1.0",
            "usage": [{
                "resource": "https://example.invalid/common-interfaces.kpar",
                "versionConstraint": "^1.2.0"
            }]
        }"#;
        let manifest: ProjectManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(manifest.name, "controller");
        assert_eq!(manifest.usage.len(), 1);
        assert_eq!(
            manifest.usage[0].version_constraint.as_deref(),
            Some("^1.2.0")
        );
    }

    #[test]
    fn defaults_to_no_usages() {
        let raw = r#"{"name":"standalone","version":"0.1.0"}"#;
        let manifest: ProjectManifest = serde_json::from_str(raw).unwrap();
        assert!(manifest.usage.is_empty());
    }
}
