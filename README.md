# syspm

_Verifiable model baselines for SysML v2 and KerML._

**Status: early development. Not yet a functional package manager.**

`syspm` is a standalone, open-source package manager for SysML v2 and KerML
model projects, independent of any particular modeling environment.

## Vision

syspm is a package manager for SysML v2 and KerML projects that puts security
and integrity first. You always know exactly what model content you depend on,
it can't change without you seeing it, and restoring it never runs anyone
else's code. It needs no central service, so it also works offline, in
air-gapped networks and under regulation.

- **Pinned by content:** a lock file records the exact bytes of every package
  a project depends on, and syspm tells you when a pinned input has changed.
- **Safe to restore:** resolving, fetching and restoring never run code from a
  package, its origin or a shared file.
- **Decentralized:** depend on local paths and Git repositories. No registry
  is required.
- **Monorepo-native:** many projects in one repository, resolved together,
  with each project usable unchanged from outside the repository and
  identified by its own content.
- **Offline:** an integrity-checked local store, and restores that are
  identical on every machine, including ones with no network.
- **Baselines you can move and prove (later):** verify a baseline offline,
  compare two baselines, and bundle one for review and transfer.

None of this is implemented yet; see [Implemented vs. planned](#implemented-vs-planned)
and the [roadmap](docs/roadmap.md).

## Implemented vs. planned

| Capability                                                           | Status                                              |
| -------------------------------------------------------------------- | --------------------------------------------------- |
| `syspm --help`, `syspm --version`                                    | Implemented                                         |
| `syspm inspect [PATH]`: show a project's name, version and usages    | Implemented, without validation                     |
| Full `.project.json` and `.meta.json` reading, discovery, validation | Planned (v0.1.0)                                    |
| `syspm init`                                                         | Planned: currently exits with "not implemented yet" |
| `syspm install`, `syspm update`                                      | Planned: currently exit with "not implemented yet"  |
| Path dependencies, lock file, sync                                   | Planned (v0.1.0)                                    |
| Git sources, verified local store, offline replay                    | Planned (v0.2.0)                                    |
| KPAR reading and writing                                             | Planned                                             |

## Design principles

- Respect the OMG KerML interchange metadata and KPAR formats. The standard
  `.project.json` is the authoritative manifest, and syspm does not invent a
  replacement. An optional `syspm.toml` may later hold syspm-specific settings.
- Never run code that a dependency selects.
- Keep package management separate from SysML semantic validation and model
  execution.
- Leave task orchestration to other tools; syspm is not a task runner.
- Make dependency restoration reproducible and verifiable.
- Start with project-level usages, not dependencies inferred from `.sysml`
  source files.
- Favor interoperability with other tools that use the standard interchange
  formats.

## Build

Requires [rustup](https://rustup.rs/); the toolchain is pinned in
`rust-toolchain.toml`.

```bash
cargo build --workspace
cargo run -p syspm -- --help
cargo run -p syspm -- inspect tests/fixtures/controller
./scripts/check.sh   # run the same checks as CI
```

## Project structure

```text
crates/syspm-cli/    `syspm` executable (package name `syspm`): argument parsing, output
crates/syspm-core/   reusable library: project metadata, future package management
tests/fixtures/      example project inputs shared by tests
docs/                architecture and roadmap
scripts/check.sh     local run of all CI quality checks
```

The CLI depends on the core; the core never depends on the CLI. See
[docs/architecture.md](docs/architecture.md).

## Roadmap

Each release is a GitHub milestone, finished when its exit check passes:

- **v0.1.0 — Local workflow:** project metadata, path dependencies, a lock
  that pins the bytes of every package, and a transactional sync that prepares
  an environment your modeling tool loads.
- **v0.2.0 — Git workflow:** Git sources pinned to exact commits, a verified
  local store, and locked and offline replay.
- **Later:** KPAR import, verification and diffing, multiple roots on
  different library versions, baselines and offline bundles.

Details: [docs/roadmap.md](docs/roadmap.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). AI coding agents should also read
[AGENTS.md](AGENTS.md).

## Licensing

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
