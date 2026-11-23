mod common;

use common::{canonical, fixture, write_file};
use std::fs;
use std::path::Path;
use syspm_core::{DiscoverError, find_project_root, list_model_files, scan_model_files};

/// Creates a project in `root`, with a model file in the `models` subdirectory.
fn write_project(root: &Path) {
    write_file(&root.join(".project.json"), "{}");
    write_file(&root.join("models/A.sysml"), "package A;\n");
}

#[test]
fn finds_root_from_the_root_itself() {
    let root = fixture("discovery");
    assert_eq!(find_project_root(&root).unwrap(), canonical(&root));
}

#[test]
fn finds_root_from_a_nested_subdirectory() {
    let root = fixture("discovery");
    let found = find_project_root(&root.join("models/deep")).unwrap();
    assert_eq!(found, canonical(&root));
}

#[test]
fn finds_root_from_a_file_in_the_project() {
    let root = fixture("discovery");
    let found = find_project_root(&root.join("models/Nested.sysml")).unwrap();
    assert_eq!(found, canonical(&root));
}

#[test]
fn finds_root_from_a_relative_start_path() {
    // Tests run with the package directory as the working directory.
    let found = find_project_root(Path::new("../../tests/fixtures/discovery/models")).unwrap();
    assert_eq!(found, canonical(&fixture("discovery")));
}

#[test]
fn finds_the_nearest_project() {
    let root = fixture("discovery/subproject");
    let found = find_project_root(&root.join("models")).unwrap();
    assert_eq!(found, canonical(&root));
}

#[test]
fn start_outside_any_project_is_error_naming_the_start_path() {
    let dir = tempfile::tempdir().unwrap();
    let error = find_project_root(dir.path()).unwrap_err();
    assert!(matches!(error, DiscoverError::ProjectNotFound { .. }));
    let name = dir.path().file_name().unwrap().to_str().unwrap();
    assert!(error.to_string().contains(name));
}

#[test]
fn missing_start_path_is_io_error_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist");

    let error = find_project_root(&path).unwrap_err();
    assert!(matches!(error, DiscoverError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));

    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(source.downcast_ref::<std::io::Error>().is_some());
    assert!(!error.to_string().contains(&source.to_string()));
}

#[test]
fn directory_named_like_the_manifest_is_not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".project.json")).unwrap();
    let error = find_project_root(dir.path()).unwrap_err();
    assert!(matches!(error, DiscoverError::ProjectNotFound { .. }));
}

#[test]
fn git_metadata_does_not_stop_the_upward_search() {
    // A repository inside a project. With no boundary, the project above it is found.
    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    let repository = dir.path().join("vendor/repository");
    fs::create_dir_all(repository.join(".git")).unwrap();
    fs::create_dir_all(repository.join("src")).unwrap();

    let found = find_project_root(&repository.join("src")).unwrap();
    assert_eq!(found, canonical(dir.path()));
}

#[test]
fn git_file_does_not_stop_the_upward_search() {
    // Worktrees and submodules have a `.git` file instead of a directory.
    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    let worktree = dir.path().join("worktree");
    write_file(&worktree.join(".git"), "gitdir: elsewhere\n");
    fs::create_dir(worktree.join("src")).unwrap();

    let found = find_project_root(&worktree.join("src")).unwrap();
    assert_eq!(found, canonical(dir.path()));
}

#[test]
fn lists_model_files_as_sorted_relative_paths() {
    let files = list_model_files(&fixture("discovery")).unwrap();
    assert_eq!(
        files,
        [
            "Core.kerml",
            "Root.sysml",
            "models/Nested.sysml",
            "models/deep/Deep.kerml",
        ]
    );
}

#[test]
fn listing_excludes_metadata_and_other_files() {
    let files = list_model_files(&fixture("discovery")).unwrap();
    assert!(!files.iter().any(|file| file.ends_with(".json")));
    assert!(!files.iter().any(|file| file.ends_with(".txt")));
}

#[test]
fn listing_skips_hidden_directories() {
    let files = list_model_files(&fixture("discovery")).unwrap();
    assert!(!files.iter().any(|file| file.contains("Hidden")));
}

#[test]
fn listing_does_not_descend_into_nested_projects() {
    let files = list_model_files(&fixture("discovery")).unwrap();
    assert!(!files.iter().any(|file| file.starts_with("subproject")));
}

#[test]
fn nested_project_lists_its_own_files() {
    let files = list_model_files(&fixture("discovery/subproject")).unwrap();
    assert_eq!(files, ["Sub.sysml", "models/SubNested.sysml"]);
}

#[test]
fn listing_applies_extensions_exactly() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "A.sysml",
        "B.kerml",
        ".draft.sysml",
        ".sysml",
        "Upper.SYSML",
        "Other.sysmlx",
        "Model.xmi",
        "Model.json",
        "sysml",
    ] {
        write_file(&dir.path().join(name), "");
    }
    let files = list_model_files(dir.path()).unwrap();
    assert_eq!(files, [".draft.sysml", "A.sysml", "B.kerml"]);
}

#[test]
fn listing_an_empty_project_gives_no_files() {
    let dir = tempfile::tempdir().unwrap();
    assert!(list_model_files(dir.path()).unwrap().is_empty());
}

#[test]
fn listing_a_missing_directory_is_io_error_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist");

    let error = list_model_files(&path).unwrap_err();
    assert!(matches!(error, DiscoverError::Io { .. }));
    assert!(error.to_string().contains(&path.display().to_string()));

    let source = std::error::Error::source(&error).expect("I/O error has a source");
    assert!(!error.to_string().contains(&source.to_string()));
}

#[cfg(unix)]
#[test]
fn listing_does_not_follow_symbolic_links() {
    use std::os::unix::fs::symlink;

    let outside = tempfile::tempdir().unwrap();
    write_file(&outside.path().join("Outside.sysml"), "package Outside;\n");

    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    // A link to a directory, a link to a model file, and a link that forms a cycle.
    symlink(outside.path(), dir.path().join("linked-directory")).unwrap();
    symlink(
        outside.path().join("Outside.sysml"),
        dir.path().join("Linked.sysml"),
    )
    .unwrap();
    symlink(dir.path(), dir.path().join("models/cycle")).unwrap();

    let files = list_model_files(dir.path()).unwrap();
    assert_eq!(files, ["models/A.sysml"]);
}

#[cfg(unix)]
#[test]
fn listing_fails_on_an_unreadable_directory() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok();
    let result = list_model_files(dir.path());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // Running as root: permissions are not enforced.
    }

    let Err(DiscoverError::Io { path, .. }) = result else {
        panic!("expected an I/O error, got {result:?}");
    };
    assert_eq!(path, locked);
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_model_file_name_is_error_naming_the_path() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(OsStr::from_bytes(b"Bad\xFF.sysml"));
    fs::write(&path, "package Bad;\n").unwrap();

    let error = list_model_files(dir.path()).unwrap_err();
    let DiscoverError::NonUtf8Path { path: reported } = error else {
        panic!("expected NonUtf8Path, got {error:?}");
    };
    assert_eq!(reported, path);
}

#[test]
fn scan_of_readable_project_is_complete() {
    let dir = fixture("discovery");
    let scan = scan_model_files(&dir);
    assert!(scan.is_complete(), "{:?}", scan.failures);
    assert_eq!(scan.files, list_model_files(&dir).unwrap());
}

#[cfg(unix)]
#[test]
fn scan_continues_past_unreadable_directory() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    write_project(dir.path());
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok();
    let scan = scan_model_files(dir.path());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return; // Running as root: permissions are not enforced.
    }

    assert!(!scan.is_complete());
    assert_eq!(scan.files, ["models/A.sysml"]);
    let [DiscoverError::Io { path, .. }] = &scan.failures[..] else {
        panic!("expected one I/O failure, got {:?}", scan.failures);
    };
    assert_eq!(path, &locked);
}

#[cfg(unix)]
#[test]
fn broken_manifest_symlink_is_a_project_root() {
    use std::os::unix::fs::symlink;

    let parent = tempfile::tempdir().unwrap();
    write_project(parent.path());

    let child = parent.path().join("child");
    fs::create_dir(&child).unwrap();

    symlink(child.join("non-existent"), child.join(".project.json")).unwrap();

    let found = find_project_root(&child).unwrap();
    assert_eq!(found, canonical(&child));
}

#[cfg(unix)]
#[test]
fn named_pipe_manifest_is_a_project_root() {
    use std::process::Command;

    let parent = tempfile::tempdir().unwrap();
    write_project(parent.path());

    let child = parent.path().join("child");
    fs::create_dir(&child).unwrap();

    // Opening a FIFO blocks until a writer appears, so this test hangs if it is read.
    let pipe = child.join(".project.json");
    assert!(
        Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );

    let found = find_project_root(&child).unwrap();
    assert_eq!(found, canonical(&child));
}
