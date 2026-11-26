# Included package tree and content digest

Selection policy `default-v1` is **provisional**: it may change in place until a
released syspm records digests. Digest format: `included-tree-v1`.

The key words MUST, MUST NOT and SHOULD are used as in RFC 2119. Rule
identifiers (S1, P1, E1, …) are stable and are used in error reports and tests.

## 1. Scope and terms

The **included package tree** is the set of files that make up one package's
content: what a content digest covers, what a captured snapshot stores, and what
is written when a package is materialized. This specification defines:

- which entries belong to it (section 2, selection);
- the canonical path and order of each member (section 3);
- which entry types are allowed and which bytes count, for directories, Git
  trees and ZIP archives (section 4);
- the content digest (section 5).

The rules never depend on the transport. Every implementation, on every
platform, gives the same members, paths, order, bytes and digest for the same
source, or the same error.

Terms:

- **Package root.** A directory that directly contains an entry named
  `.project.json` that is not a directory
  ([project-metadata.md](project-metadata.md), section 3.1).
- **Source tree.** Every entry at or below the package root, as the transport
  presents it:
  - **File system:** directory entries, listed with `lstat`, never following
    symbolic links.
  - **Git:** the tree object for the package's subdirectory at the selected
    commit (`git ls-tree -r -t`).
  - **Archive:** the central directory entries of a ZIP archive, with the
    archive root as the package root.
- **Member.** A non-directory entry that selection includes and that passes
  every check below.
- **Selection policy.** A named, versioned set of selection rules. A digest
  records the policy that produced its tree.

The included tree is not project discovery. Discovery
([project-metadata.md](project-metadata.md), section 3.3) lists the model files
that validation reports on; the included tree also contains metadata, other model
formats, documentation and assets.

### Order of evaluation

For each source, in this order:

1. Read the entries (section 4.1). Archive entries pass the archive checks first
   (section 4.5); an entry that fails one is reported and dropped.
2. Select (section 2). Entries that selection excludes are not checked further:
   a link inside `.git/` is not an error.
3. Check entry types (section 4.2) on the selected entries.
4. Check file and directory conflicts (section 4.5, A2).
5. Read and verify bytes (sections 4.3 and 4.5).
6. Check member paths (section 3) on the remaining members.

A tree with any error has no members and no digest. Warnings never change
members or the digest.

## 2. Selection: policy `default-v1`

The machine-readable form of this policy is
[policies/default-v1.json](policies/default-v1.json).

**S1. Everything is a candidate.** Every entry below the package root is a
candidate, at any depth, including hidden entries and every file type and
extension. `.project.json` and `.meta.json` are members. Ignore files such as
`.gitignore` are **not** interpreted; their own contents are ordinary members.

**S2. Nested projects are excluded.** A directory below the package root that
directly contains an entry named `.project.json` that is not a directory is the
root of another package. It and everything below it are excluded. A directory
_named_ `.project.json` does not mark a root.

**S3. Version control metadata is excluded.** An entry named exactly `.git`,
`.hg`, `.svn`, `.bzr`, `.jj`, `_darcs`, `.fslckout` or `_FOSSIL_`, file or
directory, is excluded at any depth with everything below it. Files that version
control reads but that are tracked content (`.gitignore`, `.gitattributes`,
`.gitmodules`, `.hgignore`) are not excluded.

**S4. File-manager metadata is excluded.** An entry named exactly `.DS_Store`,
`Thumbs.db`, `desktop.ini` or `__MACOSX` is excluded at any depth with everything
below it. Only whole names match; there are no patterns.

**S5. Tool settings and shared resolution files at the package root are
excluded.** `syspm.toml`, `.project-sources.json` and `.project-lock.json`
directly at the package root are excluded; anywhere else they are ordinary
members. The last two are the shared sources and lock files, which are still to
be specified. Provider-set
descriptions have no fixed name and are ordinary members wherever they are.

**S6. Nothing else is excluded.** No entry is excluded by location, extension,
size or content. Generated output and caches inside the package root, and files
that Git ignores, are members when present.

**S7. Recorded paths must be members.** A path recorded in `.meta.json` (`index`
or `checksum`) that names an entry excluded by S2–S5 is an error,
`recorded-path-excluded`, naming the path and the rule.

**Changing the policy.** `default-v1` may be edited in place until a released
syspm writes digests into a lock or baseline. From then on, any change takes a
new identifier (`default-v2`, …), and digests already recorded keep their policy.

**Git and archive sources.** A Git source considers only the package's
subdirectory tree, so a change elsewhere in the repository (including a
repository-level `.gitattributes`) cannot change the package. An archive's root
is the package root; KerML requires `.project.json` and `.meta.json` at its top
level.

## 3. Member paths

A member's **path** is its location relative to the package root: its names
joined by `/`. These rules apply to members only, after selection.

### Errors

| Rule | Error when                                                                                                           | Code                    |
| ---- | -------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| P1   | The path is not valid UTF-8                                                                                          | `invalid-utf8`          |
| P2   | A segment is empty, `.` or `..`                                                                                      | `invalid-segment`       |
| P3   | The path contains `\`                                                                                                | `backslash`             |
| P4   | The path contains a control character (general category Cc: U+0000–U+001F, U+007F–U+009F)                            | `control-character`     |
| P5   | The path contains a code point unassigned in the implementation's Unicode version (Cn, which includes noncharacters) | `unassigned-code-point` |

Each error names the path (in hexadecimal for P1) and the code.

### Canonical form and collisions

- **NFC.** A member's canonical path is the Unicode NFC form of its path. Two
  members whose paths have the same NFC form are an error,
  `normalization-collision`, naming both. The same name reaches syspm in
  different forms depending on the source: APFS keeps NFD names, Git on macOS
  records them as NFC by default, and a ZIP archive can hold both forms.
- **Case.** Two distinct canonical paths, or path prefixes (directories), at the
  same place whose segments have equal keys are an error, `case-collision`,
  reported at the shallowest level. The key of a segment is
  `NFC(toCasefold(toNFKC(segment)))` with full case folding (statuses C and F,
  not Turkic). A package whose members cannot all be written to a
  case-insensitive file system cannot be materialized faithfully on macOS or
  Windows defaults, so this is an error everywhere. The key is broad on purpose,
  and may reject a pair that one file system would accept.

### Order

Members are ordered by the **bytes of their canonical path in UTF-8** (code
point order). It does not depend on locale, platform or directory structure:
`B.sysml` sorts before `a.sysml`, `a-b.sysml` before `a/x.sysml`, and `é.sysml`
after `z.sysml`.

### Portability warnings

These never change identity. Writing such a name where it cannot exist is a
local error when writing.

| Warning                         | Condition                                                                                                                                           |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `windows-reserved-character`    | `<`, `>`, `:`, `"`, `\|`, `?` or `*` in the path                                                                                                    |
| `windows-reserved-name`         | A segment whose part before the first `.` is, in any case, `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `COM¹`–`COM³`, `LPT1`–`LPT9` or `LPT¹`–`LPT³` |
| `windows-trailing-dot-or-space` | A segment ends in `.` or space                                                                                                                      |
| `segment-too-long`              | A segment is over 255 UTF-8 bytes                                                                                                                   |
| `format-character`              | The path contains a general category Cf character, such as U+202E                                                                                   |

Total path length depends on where a package is written, so it is checked when
writing, not at capture.

### Recorded paths and Unicode version

- Paths recorded in `.meta.json` are compared with members in canonical form: a
  recorded NFD path names the NFC member.
- An implementation documents its Unicode version and rejects code points
  unassigned in it (P5). By Unicode's normalization and case-folding stability
  policies, implementations with different Unicode versions give the same
  canonical paths, order and collisions for every path both accept. An older implementation may reject a
  name a newer one accepts, but never gives a different digest.

## 4. Entries, modes, bytes and archives

### 4.1 What each source reports

| Source      | Entry types                                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------------------------- |
| File system | regular file, directory, symbolic link, named pipe, socket, character or block device                                     |
| Git         | `100644` and `100755` file, `040000` directory, `120000` symbolic link, `160000` submodule; any other mode is unsupported |
| ZIP         | a directory if the name ends in `/`, otherwise a file, unless Unix type bits say otherwise (A2)                           |

### 4.2 Entry types

| Rule | Entry                                                                                            | Result                                             |
| ---- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------- |
| E1   | Symbolic link anywhere in the selected tree: to a file, to a directory, or dangling              | Error `symbolic-link`                              |
| E2   | Named pipe, socket, device, Git submodule, any other Git mode, any other Unix type in an archive | Error `unsupported-entry`                          |
| E3   | Directory                                                                                        | Not a member; empty directories contribute nothing |
| E4   | Hard links                                                                                       | Each path is a separate regular file               |

Links are not followed, because the content would depend on the machine and
could leave the package root, and not skipped, because content would be lost
without notice. A submodule holds only a commit id; skipping it would hide
content the author meant to include.

### 4.3 Modes and bytes

- **M1.** Permissions, the executable bit, owners, timestamps and extended
  attributes are not part of identity. A written package gets default
  permissions.
- **B1.** A member's content is its exact bytes. Line endings, byte order marks
  and encodings are never converted. An empty file is a member.
- **B2.** A Git source reads blob bytes from the tree object. No
  `.gitattributes`, `core.autocrlf`, `ident`, `export-subst` or other filter is
  applied. A ZIP made by `git archive` applies checkout filters, so it can hold
  different bytes from the commit it was made from.
- **B3.** A directory source hashes the bytes on disk. When the directory is a
  Git working tree and Git considers a member unmodified (`git hash-object
  <file>`, with clean filters, equals the index or `HEAD` blob id) but its raw
  bytes differ (`git hash-object --no-filters <file>` does not), the capture
  warns `checkout-filtered`: a capture from the commit would give a different
  digest.

### 4.4 Archive references

KerML 1.1 section 10.3 says a project interchange file is "archived using the ZIP
format", that each model interchange file "shall be identifiable by a unique
path in the archive directory structure", and (Table 13, note 2) that paths are
relative to the archive root and separated by `/`. The ZIP format is PKWARE's
APPNOTE (checked against version 6.3.10).

### 4.5 Archives (ZIP)

**A1. Names.**

| Condition                                                             | Result                                                                                                                                                |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| General purpose bit 11 set                                            | The name is UTF-8 (APPNOTE MUST). Bytes that are not UTF-8 fail P1.                                                                                   |
| Bit 11 unset, every byte below 0x80                                   | Accepted (ASCII is identical in CP437)                                                                                                                |
| Bit 11 unset, a byte 0x80 or above, and the bytes are valid UTF-8     | Accepted as UTF-8. A member with such a name gets the warning `undeclared-name-encoding`.                                                             |
| Bit 11 unset, a byte 0x80 or above, and the bytes are not valid UTF-8 | Error `undeclared-name-encoding`. CP437 is never decoded.                                                                                             |
| An Info-ZIP Unicode Path extra field (0x7075) is present              | It MUST be version 1, its CRC-32 MUST match the header name, and its name MUST equal the header name bytes. Otherwise: error `unicode-path-mismatch`. |

Common archivers on macOS write UTF-8 names without bit 11 or a 0x7075 field.
Reading such bytes as UTF-8 is deterministic and never chooses between
encodings; a real CP437 name whose bytes are also valid UTF-8 gets the UTF-8
meaning. A mismatched 0x7075 field is rejected rather than ignored, so that two
readers can never extract different names from one archive.

**A2. Entries.**

| Condition                                                                                                              | Result                                                         |
| ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Two central directory entries with identical name bytes                                                                | Error `duplicate-entry`; all copies are dropped                |
| The name starts with `/`, or with a drive letter and `:` (APPNOTE 4.4.17.1)                                            | Error `absolute-path`                                          |
| General purpose bit 0 (encryption) set                                                                                 | Error `encrypted-entry`                                        |
| The local header's name differs from the central directory's                                                           | Error `local-header-mismatch`                                  |
| Host 3 (UNIX) with Unix type bits for a link, or for another non-file type                                             | E1 or E2                                                       |
| A name ending in `/` with data or with non-directory Unix type bits, or directory Unix type bits on a name without `/` | Error `inconsistent-entry-type`                                |
| A name ending in `/`                                                                                                   | A directory entry: not a member (E3), but subject to selection |
| One name is a file and also a directory: a prefix of another entry, or both `B` and `B/`                               | Error `file-directory-conflict`                                |
| Compression method other than 0 (stored) or 8 (deflate)                                                                | Error `unsupported-compression`                                |
| The CRC-32 or uncompressed size does not match, or the data cannot be inflated                                         | Error `crc-mismatch`                                           |

Parent segments and backslashes are caught by P2 and P3. Entry types come from
the name; only Unix type bits are checked against it, because other hosts'
attributes are too coarse (`git archive`, for example, writes MS-DOS attributes
for most entries). Identity is the uncompressed bytes, so compression method and
level never change it. Other methods may be accepted later, which can only turn
an error into acceptance.

**A3. Metadata.** `.project.json` and `.meta.json` are hashed as stored (section
5.3).

Resource-safety limits (entry counts, sizes, compression ratio, overlapping
data, ZIP64, multi-disk archives) are not part of identity and are specified
with safe extraction.

## 5. Content digest: format `included-tree-v1`

### 5.1 Definition

**H1.** The **manifest** is one line per member, in canonical order:

```text
<SHA-256 of the member's bytes, 64 lowercase hex digits><space><space><canonical path><LF>
```

The manifest is UTF-8, with no byte order mark and no header. Every line ends in
LF, including the last. It holds nothing else: no sizes, modes, timestamps,
directories or policy name. P3 and P4 forbid backslashes and control characters
in paths, so no line needs escaping. The **digest value** is the SHA-256 of the
manifest's bytes, as 64 lowercase hex digits.

A recorded digest has four fields (H3):

| Field       | Value                   |
| ----------- | ----------------------- |
| `format`    | `included-tree-v1`      |
| `policy`    | `default-v1`            |
| `algorithm` | `sha256`                |
| `value`     | 64 lowercase hex digits |

How the fields are written in a lock or baseline is specified with those
formats.

### 5.2 Checking with general-purpose tools

**H2.** The manifest uses the line format that `sha256sum` and `shasum -a 256`
read and write. In the package root, `shasum -a 256 -c manifest` checks every
member, and `shasum -a 256 < manifest` prints the digest value. Appendix A shows
how to produce a manifest with general-purpose tools.

### 5.3 Versioning and transports

- **H3.** Format and policy are recorded beside the value, not hashed into it, so
  that the manifest stays readable by `shasum -c`. A value recorded under the
  wrong format or policy does not match when recomputed.
- **H4.** `included-tree-v1` uses SHA-256 only. Any change to the manifest
  syntax, the line content or the algorithm takes a new `format`.
- **H5.** Every member is hashed as its raw bytes, including `.project.json` and
  `.meta.json`, which are never re-serialized. The same bytes give the same
  digest from a directory, a Git tree and an archive. A package whose
  distributions differ only in metadata bytes (for example, a `.kpar` with
  compact `.project.json` and an added `checksum` in `.meta.json`) has different
  digests in each: they are different instances, and offering both for one IRI
  and version is ambiguous
  ([package-identity.md](package-identity.md), section 3).
- **H6.** A digest is never written inside the tree it covers. Shared resolution
  files at the package root are excluded (S5). `.meta.json` checksums cover model
  files, and the digest covers `.meta.json`; that is not a cycle.
- **H7.** The Git commit and tree id, and the hash of an archive file, are origin
  data, recorded separately. They are never part of identity.

## 6. Identity and local checks

| Item                                                                          | Identity (all platforms) | Local                                  |
| ----------------------------------------------------------------------------- | ------------------------ | -------------------------------------- |
| Package root, S1–S7, policy identifier                                        | Yes                      |                                        |
| P1–P5, NFC, normalization and case collisions, order                          | Yes                      |                                        |
| E1–E4, A1, A2, member bytes (B1, B2), digest (H1–H5)                          | Yes                      |                                        |
| Portability warnings, `checkout-filtered`, `undeclared-name-encoding` warning | Warning only             | Errors when writing, where unsupported |
| Modes and permissions (M1)                                                    | Not recorded             | Default permissions when writing       |
| Total path length, resource limits                                            |                          | When writing or extracting             |

## 7. Test vectors

[tests/fixtures/included-tree/](../../tests/fixtures/included-tree/) holds the
test vectors for every section: `selection/` (section 2), `paths/` (section 3),
`entries/` with ZIP archives (section 4) and `digest/` (section 5). Each set's
expectations were written by hand or produced with general-purpose tools.

## Appendix A. Producing a manifest with general-purpose tools (informative)

For a tree that is already valid under sections 3 and 4, on a POSIX system with
`shasum`:

```sh
#!/bin/sh
# Usage: sh manifest.sh <package-root> > manifest; shasum -a 256 < manifest
set -eu
nfc=${NFC:-cat}
tab=$(printf '\t')
cd "$1"
find . -mindepth 1 \
  \( -name .git -o -name .hg -o -name .svn -o -name .bzr -o -name .jj -o -name _darcs \
     -o -name .fslckout -o -name _FOSSIL_ \
     -o -name .DS_Store -o -name Thumbs.db -o -name desktop.ini -o -name __MACOSX \) -prune -o \
  \( -type d -exec sh -c '[ -e "$1/.project.json" -o -L "$1/.project.json" ] && [ ! -d "$1/.project.json" ]' sh {} \; \) -prune -o \
  ! -type d ! -path ./syspm.toml ! -path ./.project-sources.json ! -path ./.project-lock.json -print |
  sed 's|^\./||' |
  while IFS= read -r path; do
    printf '%s\t%s\n' "$(printf '%s' "$path" | $nfc)" "$path"
  done |
  LC_ALL=C sort -t "$tab" -k1,1 |
  while IFS="$tab" read -r canonical path; do
    printf '%s  %s\n' "$(shasum -a 256 < "$path" | cut -d' ' -f1)" "$canonical"
  done
```

- It applies the `default-v1` exclusions but does not detect the errors of
  sections 3 and 4.
- Set `NFC` to a normalizer for trees whose names are not all NFC, for example
  `uconv -x any-nfc` (ICU). On macOS, `iconv -f UTF-8-MAC -t UTF-8` composes
  common accented Latin names, but its mapping is not exactly Unicode NFC.
