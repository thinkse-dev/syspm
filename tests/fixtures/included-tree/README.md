# Included-tree test vectors

Test vectors for [docs/specs/included-tree.md](../../../docs/specs/included-tree.md).
Implementations of selection, member paths, entry checks and the
`included-tree-v1` digest must reproduce every expectation here.

Each directory holds a `cases.json` whose `_comment` describes its case format.
Expected values (members, order, exclusions with their rule, errors, warnings,
manifests and digests) were written by hand, or for digests produced with
general-purpose tools (`find`, `sort`, `shasum`; the specification's appendix A).

| Directory    | Specification section                | Cases |
| ------------ | ------------------------------------ | ----- |
| `selection/` | 2, selection policy `default-v1`     | 3, plus one case that reads `tests/fixtures/discovery` in place |
| `paths/`     | 3, member paths                      | 10    |
| `entries/`   | 4, entry types, modes, bytes and ZIP archives | 21    |
| `digest/`    | 5, content digest `included-tree-v1` | 8     |

Notes:

- **Trees are described, not checked in.** Some entries (a `.git` directory,
  `.DS_Store`, links, named pipes, NFD names) cannot be stored in this repository
  faithfully, so each case is built in a temporary directory, Git repository or
  archive when tested.
- **Archives are exact.** `entries/archives/<case>.zip` is the archive for each
  archive case, built byte for byte from its specification in `cases.json`, with
  flags, attributes and extra fields set as the case requires. Do not
  regenerate them with a ZIP library.
- **Known answers.** `digest/` case `known-answers` includes SHA-256 values
  published in FIPS 180-2 (appendix B.1) and for the empty string.
- **Shared resolution files.** Cases in `selection/` and `digest/` include
  `.project-sources.json` and `.project-lock.json` at the package root, which
  `default-v1` excludes. A `selection/` case also has a `.project-lock.json`
  below the root, which is a member.
