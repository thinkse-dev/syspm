# AGENTS.md

Guidance for AI coding agents working in this repository. It summarizes the
essentials. The authoritative sources are [CONTRIBUTING.md](CONTRIBUTING.md)
for workflow and standards, [docs/architecture.md](docs/architecture.md) for
architecture, and `docs/specs/` for normative behavior once it exists. If this
file disagrees with them, they win.

## Project context

`syspm` is a standalone, open-source package manager for SysML v2 and KerML
projects, written in Rust. It puts security and integrity first: dependencies
are identified exactly, verified by content, and restored without running code
that a package selects. It works alongside modeling tools through the standard
interchange formats and does not replace them. In repository content, describe
syspm by its own capabilities; don't name or compare it with other package
managers or tools.

Status: early development. `syspm init` creates a project's `.project.json`,
and `syspm inspect` finds a project and reports its metadata, model files and
validation findings; `install` and `update` are stubs that exit 1. The plan is in [docs/roadmap.md](docs/roadmap.md).

## Architecture

- `crates/syspm-core`: library containing all package-management logic.
  Returns typed `thiserror` errors; never prints or exits.
- `crates/syspm-cli` (package and binary `syspm`): `clap` parsing, dispatch,
  output, and exit codes, using `anyhow`. Keep commands thin.
- Dependencies point only `syspm-cli` → `syspm-core`, never the reverse.
- The standard KerML `.project.json` is the authoritative manifest, and
  `.meta.json` is standard metadata. A future optional `syspm.toml` holds only
  settings specific to syspm; future sources and lock files, in documented,
  tool-neutral formats, record resolution intent and its result. None of these
  exist yet.
- Package identity is the usage IRI, versions are opaque strings, and only
  exact version constraints are supported
  ([docs/specs/package-identity.md](docs/specs/package-identity.md)).
- Do not invent KerML or SysML requirements. Version semantics beyond exact
  constraints, resolution, source and lock formats, registries and KPAR
  details are **open** until they are specified in `docs/specs/`. Ask, or
  record an assumption, rather than guessing.
- No speculative abstractions, traits, or crates ahead of a feature that
  needs them.

## Development standards

- Idiomatic Rust, edition 2024, using the toolchain pinned in
  `rust-toolchain.toml`.
- Errors: a message says what failed and includes context (such as the path);
  the cause goes in `source`, not in the message. The CLI prints
  `syspm: {error:#}` to stderr. Exit codes are 0 for success, 1 for failure,
  and 2 for usage errors. Avoid `unwrap`/`expect` outside tests.
- Every behavior change comes with tests. CLI tests run the real binary
  (`crates/syspm-cli/tests/cli.rs`); shared inputs go in `tests/fixtures/`.
- Before you report work as done, run `./scripts/check.sh` (actionlint,
  dprint, rustfmt, Clippy with `-D warnings`, tests, build, and the
  `cargo deny` dependency audit).
- Commits and PR titles follow Conventional Commits: `type(scope)?: description`.

## Agent behavior

- Read the existing code and docs before changing them, and preserve
  established design rules. If you think one is wrong, raise it; don't silently
  override it.
- Make small, focused, independently verifiable changes. Add no new
  dependency without a concrete need, and say why you added it.
- Add or update tests for any changed behavior, and run the relevant checks.
  Never claim a check passed unless you ran it and saw it pass. Report
  failures and skipped steps.
- Write down assumptions and unresolved questions in your summary, or in an
  issue when they affect design.
- Don't commit, push, tag, rewrite Git history, change GitHub settings, or
  create releases unless explicitly asked.
