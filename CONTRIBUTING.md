# Contributing to syspm

Thanks for your interest in syspm. The project is in early development, so the
workflow is intentionally lightweight and will tighten as it grows.

## Development setup

### Required tools

| Tool              | Purpose                                   | Install (macOS)                                                  |
| ----------------- | ----------------------------------------- | ---------------------------------------------------------------- |
| rustup            | Rust toolchain, `rustfmt`, `clippy`       | <https://rustup.rs/>                                             |
| Lefthook          | Git hook manager                          | `brew install lefthook`                                          |
| Cocogitto (`cog`) | Conventional Commit validation            | `cargo install --locked cocogitto` (or `brew install cocogitto`) |
| dprint            | Formatting for Markdown, TOML, JSON, YAML | `cargo install --locked dprint` (or `brew install dprint`)       |
| actionlint        | GitHub Actions workflow linting           | `brew install actionlint`                                        |
| cargo-deny        | Dependency advisories, licenses, sources  | `cargo install --locked cargo-deny@0.20.2` (the version CI uses) |

No Node.js is needed.

Without Homebrew, download Lefthook and actionlint from their release pages
([Lefthook](https://github.com/evilmartians/lefthook/releases),
[actionlint](https://github.com/rhysd/actionlint/releases)), verify the
checksums, and put the binaries on your `PATH`.

### First-time setup

```bash
# Toolchain: rust-toolchain.toml pins the version and components; rustup
# installs them on the first cargo command. To do it explicitly:
rustup toolchain install

cargo install --locked cocogitto dprint
cargo install --locked cargo-deny@0.20.2  # match CARGO_DENY_VERSION in ci.yml
brew install lefthook actionlint

# Enable the Git hooks (re-run after changing lefthook.yml, or to repair hooks)
lefthook install
```

### Rust version policy

- **Edition:** 2024, set once in the root `Cargo.toml`.
- **Toolchain:** pinned to an exact stable release in `rust-toolchain.toml`
  (currently 1.99.1). Local builds and CI use the same compiler, so formatting
  and Clippy results match. Nightly is not used.
- **MSRV:** `rust-version` in the root `Cargo.toml` (currently 1.99) is the
  minor version of the pinned toolchain. CI builds only with the pinned
  toolchain, so no older compiler is claimed to work. Lowering the MSRV
  requires adding a CI job that builds with that version.
- **Upgrading:** bump `rust-toolchain.toml` and `rust-version` together in one
  `build:` commit, and fix any new Clippy findings in the same PR.
- **`Cargo.lock`** is committed (this workspace ships a binary). CI and hooks
  use `--locked`, so dependency changes must include the updated lockfile.

### Editor

The repository includes `.vscode/settings.json`, which turns on format-on-save,
formats Rust with rust-analyzer and Markdown, TOML, JSON, and YAML with dprint,
and makes rust-analyzer run Clippy. The recommended extensions, including the
dprint extension, are listed in `.vscode/extensions.json`.

### Everyday commands

```bash
cargo fmt --all                                         # format all Rust code
cargo fmt --all -- --check                              # check formatting only
dprint fmt                                              # format Markdown/TOML/JSON/YAML
dprint check                                            # check them only
actionlint                                              # lint .github/workflows
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny --locked check                               # dependency policy (deny.toml)
cargo run -p syspm -- --help
./scripts/check.sh                                      # everything CI runs
```

## Code quality

- **Formatting:** Rust uses the default `rustfmt` style with no overrides.
  Markdown, TOML, JSON, and YAML use dprint (`dprint.json`), which excludes
  `Cargo.lock`, `target/`, and `tests/fixtures/`. `.editorconfig` matches both
  formatters. Format before committing; the pre-commit hook checks formatting
  but never rewrites files.
- **Linting:** Clippy's default lints, with every warning treated as an error.
  Fix the code rather than adding `#[allow(...)]`. If an allow is genuinely
  needed, scope it as narrowly as possible and add a comment explaining why.
- **Tests:** every change comes with tests. Put integration tests in each crate's `tests/`
  directory and shared inputs in `tests/fixtures/`. Don't write tests for
  features that don't exist yet.
  - Unit tests go in a `#[cfg(test)] mod tests { … }` block at the end of the
    module they test.
  - When the tests make the module hard to read, move them to
    `<module>/tests.rs` and declare them with `#[cfg(test)] mod tests;`.
- **Errors:**
  - `syspm-core` returns typed errors derived with `thiserror`, so callers can
    match on the failure. Add a variant only when a caller needs to tell that
    case apart. The library never prints or exits.
  - An error's message describes _what_ failed and includes context such as
    the path. The underlying cause goes in a `source` field and is **not**
    repeated in the message (`#[error("cannot read project manifest {}",
    path.display())]`, not `"...: {source}"`).
  - `syspm-cli` uses `anyhow::Result`. Add `.context(...)` where it helps the
    user, and use `bail!` for CLI-level failures. `main` prints `syspm:
    {error:#}` (the whole cause chain on one line) to stderr.
  - Output: a command's results go to stdout, and execution failures and fatal
    summaries go to stderr. A validation finding is a result, so it goes to
    stdout even when it makes the command fail. Exit codes are `0` for success,
    `1` for a failed command, and `2` for a usage error (from clap).
  - Avoid `unwrap()`/`expect()` in production paths where failure is
    realistically possible (they are fine in tests).
- **Architecture:** package-management logic belongs in `syspm-core`.
  `syspm-cli` is limited to argument parsing, dispatch, and output. Read
  [docs/architecture.md](docs/architecture.md) before you change architecture,
  and propose design changes as described in
  [Proposing design changes](#proposing-design-changes).
- **Dependencies:** prefer the standard library. Add a crate only when there is
  a concrete need, declare shared versions in the root
  `[workspace.dependencies]`, and commit `Cargo.lock`. Every dependency must
  pass the [dependency policy](#dependency-policy).
- Don't add abstractions ahead of a feature that uses them.

## Proposing design changes

Open an issue before you start on a change to behavior, file formats, or the
architecture. Describe the problem, the change you propose, and the
alternatives you considered. Small fixes and changes that follow an existing
specification don't need one.

- **Normative rules go in `docs/specs/`:** anything another implementation
  would need to reproduce syspm's results, such as a file format, a digest, or
  an error code. Each rule has an identifier that tests can refer to. Change
  the specification in the same PR as the code, or before it.
- **Architecture goes in [docs/architecture.md](docs/architecture.md):** crate
  boundaries, design rules, and deliberately deferred decisions. Moving an item
  out of "Deliberately deferred decisions" needs an issue first.
- **Plans go in [docs/roadmap.md](docs/roadmap.md)** and the GitHub
  Milestones.

## Dependency policy

`deny.toml` sets the policy, and `cargo deny --locked check` enforces it: in CI
on every PR and push to `main` and weekly on `main`, and locally in
`./scripts/check.sh`. It needs network access to fetch the RustSec advisory
database.

| Check      | Fails on                                                                                        | Warns on                         |
| ---------- | ----------------------------------------------------------------------------------------------- | -------------------------------- |
| advisories | A vulnerability, unmaintained, or unsound advisory for any crate in the graph; a yanked version |                                  |
| licenses   | A license that is not allowed (Apache-2.0, MIT, Unicode-3.0)                                    | An allowed license nothing uses  |
| bans       | A registry dependency without a version requirement                                             | More than one version of a crate |
| sources    | Any registry other than crates.io, or any Git source                                            |                                  |

The dependencies of every platform are checked, not only the one you build on.
Dev-dependencies are checked for advisories and sources but not licenses,
because they are not part of the binary.

The advisory database changes daily, so a PR can fail on an advisory it did not
cause. Fix it in that PR or in a separate one first, as described below.

### Handling a finding

- **Vulnerability, unsound, or yanked:** update the affected crate to a fixed
  version, usually with `cargo update -p <crate>`, in a `build(deps):` commit,
  and run `./scripts/check.sh`. If the fix needs a new major version of a
  direct dependency, treat it like any other dependency change.
- **Unmaintained:** open an issue to replace the crate. Ignore the advisory in
  the meantime only as described below.
- **No fix available:** ignoring an advisory is the last resort and needs a
  maintainer's approval in the PR. Add one entry to `[advisories] ignore`,
  naming the advisory ID, never a whole crate, with a `reason` that says why
  syspm is not affected or why the risk is accepted, and links the issue that
  tracks its removal. Remove the entry once a fix is available.
- **License:** allowing a new license is a policy change that needs a
  maintainer's approval. Check that it is compatible with distributing syspm
  under MIT OR Apache-2.0. When only one crate needs it, prefer an exception
  for that crate (`exceptions = [{ crate = "...", allow = ["..."] }]`) over
  adding it to `allow`.
- **Source:** dependencies come from crates.io. Any other source needs a
  maintainer's approval and an `allow-registry` or `allow-git` entry naming
  exactly that source.
- **Duplicate versions:** these are warnings. Remove them when an update makes
  it easy, and don't add `skip` entries just to silence them.

Never disable or narrow a whole check, for example with `unmaintained = "none"`,
to make a build pass.

### Updating cargo-deny

Dependabot does not update cargo-deny. To update it:

1. In `.github/workflows/ci.yml`, set `CARGO_DENY_VERSION` and
   `CARGO_DENY_SHA256` together. The checksum is in the release's
   `cargo-deny-<version>-x86_64-unknown-linux-musl.tar.gz.sha256` file.
2. Update the version in the install commands in this file.
3. Install the new version locally and run `./scripts/check.sh`.

## Git workflow

### Phase 1: Bootstrap (complete)

Direct commits to `main` were acceptable, and Issues and PRs were optional.

### Phase 2: Feature development (current)

Simplified trunk-based development:

- `main` is the only permanent branch.
- Create short-lived branches from an up-to-date `main`.
- Associate meaningful work with a GitHub Issue, and use its native issue number.
- Assign the issue to the GitHub Milestone (`vX.Y.Z — <name>`) it belongs to,
  if any; see [docs/roadmap.md](docs/roadmap.md).
- Open pull requests that target `main`.
- Squash-merge, then delete the branch.

Branch names:

```text
feat/<issue-number>-<description>
fix/<issue-number>-<description>
docs/<issue-number>-<description>
chore/<issue-number>-<description>
```

Examples: `feat/12-inspect-command`, `fix/18-manifest-path-error`,
`docs/21-installation-guide`, `chore/24-update-ci`.

PR descriptions should reference the relevant issue, using `Closes #N` when the
PR completes it. Small documentation fixes and minor maintenance don't always
need an issue.

There is no `develop` branch and there are no release branches.

### Phase 3: Community contributions

As the contributor base grows, the project may introduce branch protection,
required CI checks, mandatory code review, and a formal release procedure.

## Pull requests

1. Start from an issue (or a clear description, for small fixes) and a branch
   off an up-to-date `main`.
2. Run `./scripts/check.sh` before pushing.
3. Open the PR against `main`. The template asks for a summary, `Closes #N`,
   and how the change was verified.
4. Use a Conventional Commit as the PR title; it becomes the squash commit.
5. Merge once CI passes (see [Checks](#checks-local-vs-ci)), then delete the
   branch.

## Commit conventions

Commit messages and PR titles follow
[Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/):

```text
<type>(optional-scope): description
```

| Type       | Use for                                     |
| ---------- | ------------------------------------------- |
| `feat`     | New functionality                           |
| `fix`      | Bug fixes                                   |
| `docs`     | Documentation                               |
| `test`     | Tests                                       |
| `refactor` | Code restructuring without behaviour change |
| `perf`     | Performance improvements                    |
| `build`    | Build system, Cargo, dependencies           |
| `ci`       | Continuous integration                      |
| `chore`    | Maintenance                                 |
| `style`    | Formatting-only changes                     |
| `revert`   | Reverting a previous change                 |

- **Scopes are optional** and unrestricted. Common ones are `cli`, `core`, and
  `docs`.
- **Issue numbers are not required** in commit messages; put them in the PR.
- **Breaking changes:** add `!` after the type or scope (`feat(core)!: ...`),
  or add a `BREAKING CHANGE: <description>` footer. PR titles are a single
  line, so they must use `!`.
- After `git revert`, rewrite Git's default `Revert "..."` message as
  `revert: <original description>`.
- `fixup!`, `squash!`, and merge commits are accepted locally; they disappear
  when the PR is squash-merged.

Examples:

```text
feat(cli): add inspect command
fix(core): handle missing project manifest
refactor(core)!: return typed errors from read_project
docs: update architecture guide
```

### Using Cocogitto

```bash
cog commit feat "add inspect command" cli   # type, description, optional scope
cog commit fix "handle missing manifest" -B # -B marks a breaking change
cog commit docs "update guide" -e           # -e opens your editor to add a body/footer
cog verify "feat(cli): add inspect"         # check a message without committing
```

`cog commit` only commits what is already staged. A plain `git commit` works
just as well, because the `commit-msg` hook validates the message either way.

## Checks: local vs. CI

| Check                              | pre-commit | commit-msg | pre-push | CI (`ci.yml`) | CI (`pr-title.yml`) |
| ---------------------------------- | :--------: | :--------: | :------: | :-----------: | :-----------------: |
| actionlint (when workflows change) |     ✓      |            |          |       ✓       |                     |
| `dprint check`                     |     ✓      |            |          |       ✓       |                     |
| `cargo fmt --all -- --check`       |     ✓      |            |          |       ✓       |                     |
| Commit message (Cocogitto)         |            |     ✓      |          |               |                     |
| Clippy, `-D warnings`              |            |            |    ✓     |       ✓       |                     |
| `cargo test --workspace --locked`  |            |            |    ✓     |       ✓       |                     |
| `cargo build --workspace --locked` |            |            |          |       ✓       |                     |
| `cargo deny --locked check`        |            |            |          |       ✓       |                     |
| PR title is a Conventional Commit  |            |            |          |               |          ✓          |

Two more automations run on PRs. Neither is a merge requirement:

- **Claude Code Review** (`claude-code-review.yml`) posts inline review
  comments on non-draft PRs from branches in this repository. Treat them as
  suggestions.
- **Dependabot** (`.github/dependabot.yml`) opens weekly grouped update PRs
  for GitHub Actions (`ci(deps): ...`) and Cargo (`build(deps): ...`),
  ignoring releases less than 14 days old. Review them like any other PR. The
  Rust toolchain, dprint plugins, and cargo-deny are bumped manually; for the
  plugins, run `dprint config update`, then `dprint fmt`, and for cargo-deny,
  see [Updating cargo-deny](#updating-cargo-deny).

`ci.yml` also runs every week on `main`, so that an advisory published after a
merge is reported without waiting for the next change.

Local hooks are a convenience that gives fast feedback; they can be skipped
(`git commit --no-verify`, `LEFTHOOK=0 git push`) and nothing guarantees they
are installed. **CI is the authoritative check.** CI doesn't validate
intermediate commit messages: with squash merging, the PR title becomes the
commit on `main`.

A PR is ready to merge when:

1. The `Rust CI` and `PR title` checks pass.
2. The title is a Conventional Commit that accurately describes the whole
   change.
3. The description references the relevant issue (`Closes #N`).

### Troubleshooting

- **pre-commit fails on formatting:** run `cargo fmt --all` and/or
  `dprint fmt`, then `git add` the changes. Hooks check the working tree, so
  unstaged formatting problems also block the commit.
- **commit-msg rejects the message:** the error points to the problem. Fix the
  message, or check it first with `cog verify "<message>"`. If you use
  `git commit -m`, your message is still in the shell history.
- **`cargo deny` fails:** see [Handling a finding](#handling-a-finding).
- **`cog`, `lefthook`, `dprint`, `actionlint`, or `cargo deny`: command not found:** install the
  tool (see above) and make sure `~/.cargo/bin` or the Homebrew prefix is on
  your `PATH`.
- **Hooks aren't running:** run `lefthook install`. To debug,
  `lefthook run pre-push` runs a hook manually.
- **Clippy fails in CI but not locally:** CI uses the toolchain pinned in
  `rust-toolchain.toml`. Run `rustup show` to confirm you're on it too.
- **`--locked` fails:** `Cargo.lock` is out of date. Run `cargo build`, then
  commit the updated lockfile.
- **PR title check fails:** edit the PR title; the check re-runs automatically.

## Versioning and releases

- Versions are [SemVer](https://semver.org/) identifiers. The workspace has a
  single version (`[workspace.package] version` in the root `Cargo.toml`) that
  all crates inherit, so they stay consistent.
- Before 1.0, any release may contain breaking changes. Each milestone in
  [docs/roadmap.md](docs/roadmap.md) is a minor version, named after its release
  tag (v0.1.0, v0.2.0, …); fix-only releases between milestones bump the patch
  version (v0.1.1).
- `version` in `Cargo.toml` is the last released version. A maintainer bumps
  it in the PR that prepares a release, not before.
- A release is an annotated Git tag `vX.Y.Z` on `main`
  (`git tag -a v0.1.0 -m "syspm 0.1.0"`), created by a maintainer when a
  milestone's issues are closed and its exit criterion is met. A tag is not a
  binary distribution; no binaries are published yet.
- Both crates are published to crates.io by a maintainer, by hand, from a
  release tag. No version bumps, changelog generation, or publishing are
  automated yet. Later, Conventional Commit history on `main` can drive
  `cog bump` and `cog changelog`. That will be adopted deliberately; see
  [docs/roadmap.md](docs/roadmap.md).

## License

syspm is licensed under either of [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT), at your option. Unless you explicitly state otherwise, any
contribution you intentionally submit for inclusion in syspm, as defined in
the Apache-2.0 license, is dual licensed as above, without any additional
terms or conditions.
