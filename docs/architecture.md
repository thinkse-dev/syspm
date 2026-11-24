# Architecture

This document records what is **decided** today and what is **planned or
deferred**. Anything not listed under "Design rules" should be treated as open.

## Purpose and scope

`syspm` is a standalone, open-source package manager for SysML v2 and KerML
projects that puts security and integrity first. A project's dependencies are
identified exactly, verified by their content, and restored without running
code that a package contains or refers to. Reproducible, verifiable model
baselines follow from this, including in offline and restricted environments
and in monorepo workspaces that hold many projects. syspm is independent of
any modeling environment: it works on the standard project files that modeling
tools read and write.

In scope (eventually): reading and validating standard project metadata,
resolving and locking project dependencies, fetching and caching packages, and
reading and writing standard interchange archives (KPAR).

Out of scope: SysML and KerML semantic analysis and validation (left to
modeling tools, which syspm can hand a prepared environment to), general task
orchestration, and model editing.

## System context

```text
user / CI ──▶ syspm (CLI) ──▶ syspm-core ──▶ project files on disk
                                  │           (.project.json, .meta.json)
                                  └──(planned)──▶ package sources: local paths, Git, archives
modeling tools read and write the same standard files
```

Only local files are read today, and only `.project.json`. Everything marked
"planned" is on the [roadmap](roadmap.md), not implemented.

## Design rules

- **Language:** Rust, Cargo workspace, edition 2024, pinned toolchain
  (`rust-toolchain.toml`).
- **Executable:** a single CLI binary named `syspm`, with argument parsing via
  `clap` (derive).
- **Crate split:** `syspm-cli` (user interface) and `syspm-core` (reusable
  library). A library lets editor integrations and other tools use the same
  behavior as the CLI, without parsing its output.
- **Authoritative manifest:** the standard KerML `.project.json` file. syspm
  does not invent a replacement for it, so a project stays usable by every tool
  that reads the standard.
- **Standard metadata:** `.meta.json` is part of the standard interchange
  format, and is read and written as the standard defines.
- **No implicit execution:** reading, resolving, fetching and restoring a
  package never run code that the package, its origin or a shared file
  selects. Validation runs only a tool the user chose.
- **No task runner:** task scheduling, caching and affected-work execution are
  left to external tools. syspm can report which projects a change affects,
  but it does not run work.

## Crates and dependency direction

```text
syspm-cli  ──depends on──▶  syspm-core
(binary: syspm)             (library)
```

`syspm-core` must never depend on `syspm-cli`.

### `syspm-cli` (package `syspm`)

- Parses arguments, dispatches commands, formats user-facing output, and maps
  errors to exit codes.
- Holds no package-management logic of its own. A command is a thin function
  that calls into `syspm-core` and prints the result.
- Exit codes: `0` success, `1` command failure (including "not implemented
  yet"), `2` usage error (emitted by `clap`).
- Results go to stdout. Stderr is for execution failures.

`inspect [PATH]` finds the project at or above `PATH` (default: the current
directory) and prints its manifest, its metadata, its model files and every
validation finding. It exits `0` when there are no errors (warnings are
allowed), and `1` on an error finding or a `.meta.json` that can't be read.
When `.meta.json` can't be read, or some directories can't be listed, it still
prints what it could find and marks those parts as partial. `init`, `install`
and `update` exit `1` with "not implemented yet".

The package is named `syspm` so that `cargo run -p syspm` and the binary name
match; the directory is `crates/syspm-cli` to make its role explicit.

### `syspm-core`

- Reusable library for any front end: the CLI, editor integrations, or other
  tools.
- Exposes typed errors (`thiserror`, for example `ReadManifestError`) rather
  than printing; the CLI adds context and reports them with `anyhow`.
- Reads every Table 12 property of `.project.json` (`read_manifest`) and every
  Table 13 property of `.meta.json` (`read_metadata`), checking JSON structure
  and types only.
- Validates both files against the spec and the files on disk
  (`validate_manifest`, `validate_metadata`). It reports every problem at once
  as a `Diagnostic` with a stable rule identifier and a severity, and verifies
  `SHA256` checksums. Other checksum algorithms are reported as not verified.
- Finds the project root from any path inside the project
  (`find_project_root`), and lists its `.sysml` and `.kerml` files in a stable
  order (`list_model_files`, or `scan_model_files` to carry on past unreadable
  directories).

New abstractions are added to `syspm-core` when a concrete command needs them,
not in anticipation.

## Project files

| File                    | Status                 | Owner                 | Purpose                                                                                                                           |
| ----------------------- | ---------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `.project.json`         | Standard, read         | User / modeling tools | Project identity and declared dependencies (`usage`)                                                                              |
| `.meta.json`            | Standard, read         | Tools                 | Standard interchange metadata                                                                                                     |
| `.project-sources.json` | **Planned**            | User                  | Shared resolution intent: which projects are roots, where each package comes from, and the selected standard-library provider set |
| `.project-lock.json`    | **Planned**            | Package manager       | Exact, deterministic result of resolution, pinning the bytes of every package. Not hand-edited                                    |
| `syspm.toml`            | **Proposed, optional** | User                  | syspm-specific settings only (for example cache settings). Must never duplicate or override what `.project.json` declares         |

How `.project.json` and `.meta.json` are found, read and validated is
specified in [specs/project-metadata.md](specs/project-metadata.md).

The sources and lock files use tool-neutral formats, so that another tool can
read them and reproduce a result. Their formats will be specified in
`docs/specs/` before they are implemented. `syspm.toml` holds nothing another
tool would need to reproduce a result.

## Planned package-management architecture

Future work is expected to separate:

1. **Project discovery and metadata:** find and read `.project.json` and
   `.meta.json`, and validate them.
2. **Resolution:** turn declared usages into a concrete set of packages, with
   one closure per root project.
3. **Locking:** record the resolution, including a content digest of each
   package, in a tool-neutral lock file.
4. **Sources and fetching:** local paths, then Git, then archives.
5. **Store and cache:** integrity-verified, content-addressed package storage.
6. **Archives:** KPAR reading and writing.

| Area                                               | Owner                            |
| -------------------------------------------------- | -------------------------------- |
| CLI user interaction                               | `syspm-cli`                      |
| Interchange metadata and future KPAR handling      | `syspm-core`                     |
| Package source resolution and version constraints  | `syspm-core`                     |
| Integrity verification and content-addressed cache | Future store component           |
| Task scheduling, caching and affected work         | External tools                   |
| SysML semantic analysis and validation             | External modeling tools          |
| Model repository and search                        | Separate future registry project |

These may become modules of `syspm-core` or separate crates; that decision is
deferred until there is code to split.

### Guiding principles

1. A distributable model project is distinct from the SysML namespaces
   declared in its source files.
2. Project dependencies come from the standard project usage declarations.
   References between model elements need semantic analysis, which syspm does
   not do.
3. The lock file and shared source bindings use documented, tool-neutral
   formats; the cache is implementation-specific. None of them replaces the
   interchange standard.
4. Two versions that export the same SysML qualified names are not assumed to
   load together in one model context.
5. KPAR interoperability is not claimed until it has been checked against
   normative fixtures and at least one independent implementation.
6. The first version uses no database and no background service.

### Delivery order

Releases, their order, and their exit checks are in [roadmap.md](roadmap.md).

## Known constraints

- Validation checks what the spec lists, not full conformance to the
  interchange schema, and not that `index` names match the elements in each
  file.
- Of the checksum algorithms the standard allows, only `SHA256` is verified.
- No network access, background service, or database is used.
- The only supported toolchain is the pinned one (see
  [CONTRIBUTING.md](../CONTRIBUTING.md)).

## Deliberately deferred decisions

These are **open** and must not be treated as designed:

- Mapping between packages and SysML and KerML namespaces.
- Version constraint semantics beyond exact equality, and the resolution
  algorithm.
- Registry sources, protocol and hosting.
- The `syspm.toml` format.
- Cache layout.
- Whether `syspm-core` is split into further crates.

Also deferred: a remote registry, a web UI, a full semantic model parser,
editor plugins, a task runner, semantic interface diffing, package signing,
and automatic dependency inference.
