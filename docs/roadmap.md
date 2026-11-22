# Roadmap

syspm is a package manager for SysML v2 and KerML projects that puts security and integrity first. You always know exactly what model content you depend on, it can't change without you seeing it, and restoring it never runs anyone else's code. It needs no central service, so it also works offline, in air-gapped networks and under regulation.

This is direction only. Each release is a GitHub milestone, and work is tracked as issues in it. A release is done when its exit check passes. These are gates, not dates.

Every release is for Windows, macOS and Linux, tested in CI on all three. Each one ships binaries, updated documentation, and a list of what it does not yet do.

## v0.1.0 — Local workflow

For a person or small team working with textual models on one machine. Given explicit path dependencies, syspm prepares an environment that your modeling tool loads, and tells you when a pinned input has changed.

- **Project metadata:** read and validate `.project.json` and `.meta.json`, find the project, `syspm inspect` and `syspm init`.
- **Specifications:** package identity, the content digest, the sources file, provider sets, the lock, freshness, lock and network modes, safety boundaries and validator integration, with fixtures for each.
- **Resolution:** path dependencies, including transitive ones, with clear errors for cycles, conflicts and missing packages.
- **Lock:** pins the bytes of every package, with one closure per root project.
- **Sync:** transactional, so a failed or interrupted sync keeps the last good state.
- **Output:** the environment and dependency graph as JSON.
- **Commands:** add, remove and check dependencies; set up a standard library from a local directory; generate and update `.meta.json`.
- **Release:** binaries and an install path, a tutorial for the local workflow, the supported platforms, and a short security model saying what syspm protects, from what, and what it does not guarantee. The CLI, exit codes and lock format are documented and marked unstable before 1.0.

**Exit:** a project and its transitive library load in a modeling tool; locked path drift fails; a repeated sync does nothing; an interrupted sync keeps the last good state; and an outside user completes init → add → lock → sync → load from the documentation on each supported platform.

**Not in v0.1.0:** Git sources, offline replay from a store, KPAR archives, and any registry.

## v0.2.0 — Git workflow

Share libraries through Git, and reproduce them exactly on another machine, including one with no network.

- Git sources pinned to an exact commit, with tags and branches recorded as intent, and projects in repository subdirectories identified by their own content.
- Restricted Git use: no repository-configured helpers, filters, submodules or LFS run.
- A verified local store, with locked and offline replay.
- Standard-library setup from a Git checkout.
- An updated tutorial covering Git dependencies.

**Exit:** a relocated project reproduces its locked bytes from the store with the network denied; a moved tag cannot change a locked sync; and a commit elsewhere in a repository does not change a subproject's identity.

## Planned (Not Exhaustive)

Not yet assigned to versions. In order:

1. **KPAR import:** archive sources from a local file or an HTTPS URL, with bounded extraction; standard-library setup from release archives; a diagnostic for a version constraint that is not exact, and a root-level override.
2. **Verify, diff and multiple roots:** verify against the lock without network access; diff two locks; explain why a package is present; list the projects a change affects; and roots on different library versions in one workspace.
3. Baseline capture, and an offline bundle that can be imported and restored.
4. Local source and host policy.
5. A model inventory; signature verification only if a user needs it.
6. Fuller affected-project analysis, focused updates and validation evidence.
7. KPAR export.
8. Version range experiments.
9. Broader integrations and platform testing, then the 1.0 compatibility promise.
