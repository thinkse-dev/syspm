# Project metadata

This covers how syspm finds a project, reads `.project.json` and `.meta.json`, and validates them. It is based on section 10.3 of KerML 1.0 and the normative schema, `KerML-Model-Interchange.json`. In the few places where the prose and the schema disagree we follow the schema, and the metadata OMG ships with its standard libraries.

Section numbers and rule identifiers below won't be renumbered, so tests and other docs can cite them.

## 1. The two files

`.project.json` is the manifest. It holds the project's identity and the projects it uses (Table 12), and syspm reads those from nowhere else. syspm has no manifest of its own and never writes anything that duplicates or overrides `.project.json`.

`.meta.json` is the optional interchange metadata from Table 13. syspm reads it the way the standard describes and doesn't give it any extra meaning.

Both live in the project root, and both are opened only if they are regular files. A symbolic link is rejected even if it points at a valid file, and so are pipes and devices. If `.meta.json` is one of these it is treated as unreadable. It doesn't count as missing, because otherwise a broken link would look like a project without metadata.

## 2. Reading

### 2.1 Properties

`.project.json` (Table 12):

| Property      | Type             | Required |
| ------------- | ---------------- | -------- |
| `name`        | string           | yes      |
| `description` | string           | no       |
| `version`     | string           | yes      |
| `license`     | string           | no       |
| `maintainer`  | array of strings | no       |
| `website`     | string           | no       |
| `topic`       | array of strings | no       |
| `usage`       | array of usages  | no       |

Each usage is an object with a required `resource` string and an optional `versionConstraint` string. (This is a dependency between projects. It has nothing to do with usage elements inside a model.)

`.meta.json` (Table 13):

| Property          | Type                       | Required |
| ----------------- | -------------------------- | -------- |
| `index`           | object, name to path       | yes      |
| `created`         | string                     | yes      |
| `metamodel`       | string                     | no       |
| `includesDerived` | boolean                    | no       |
| `includesImplied` | boolean                    | no       |
| `checksum`        | object, path to a checksum | no       |

A checksum is an object with two required strings, `value` and `algorithm`.

Table 13's text says each `checksum` entry is a list of checksums. The schema says it's a single object, and every OMG file we've looked at agrees with the schema, so that's what syspm expects.

### 2.2 When a file can't be read

The following make a file unreadable. They are reported as a read failure, before any of the rules in section 4 run.

1. Anything other than an object where the tables expect one (the file itself, a usage, a checksum). Arrays are not accepted in place of objects.
2. A property with the wrong type. Optional properties can be left out, but an explicit `null` is a wrong type.
3. The same name twice in one object. This applies everywhere, including the keys of `index` and `checksum` and objects nested inside unknown properties. Different readers keep different duplicates, so a file like that doesn't have one meaning.

Properties the schema doesn't define are fine. The schema allows them and some tools write them.

## 3. Finding the project

### 3.1 The root

Starting from a path, syspm walks up until it finds a directory with an entry called `.project.json` that isn't itself a directory. That directory is the root, returned as a canonical path.

The entry doesn't have to be a readable file. A broken link or a pipe still marks the root, so the problem gets reported instead of syspm quietly moving on to some parent project.

Some details:

- The start can be relative and can be a file.
- If the start is a `.project.json` that isn't a directory, the walk begins at its parent. A link there is not followed, so it fails the check in section 1 instead of pulling in whatever project it points to.
- The walk only stops at the file-system root. A `.git` directory or file doesn't stop it, which means projects can sit anywhere in a repository, worktrees and submodules included.
- If there's no root, the error names the start path.

### 3.2 Nested projects

A directory inside a project with its own `.project.json` is a separate project. Starting a search inside it finds that project. The outer project's file listing skips it, and paths in the outer `.meta.json` can't reach into it (section 4.3). So each model file belongs to exactly one project.

### 3.3 Model files

A project's model files are the files under its root with the extension `sysml` or `kerml`, matched exactly. They're listed relative to the root with `/` separators and sorted by byte value.

Hidden directories, symbolic links and nested projects are skipped. Hidden files are listed if they have a model extension. JSON and XMI files aren't listed (`.json` would catch all sorts of unrelated files), though `.meta.json` can still name them. A file whose name isn't valid UTF-8 is a failure, since metadata can't record it.

If part of the tree can't be read, a caller asking for the full list gets an error. Validation keeps going instead. It reports each failure as `file-unreadable` and still checks the files it could reach. It doesn't guess at what's in a directory it couldn't open.

## 4. Validation

### 4.1 Diagnostics

Validation collects every problem rather than stopping at the first. A diagnostic has a path (relative to the root, `/` separators, `.` for the root itself), a rule identifier, a level (error or warning, decided by the rule) and a message. Messages are for people and may change.

Diagnostics are sorted by path, then by rule in the order of the table below, so output is stable between runs. Problems in a project always come back as diagnostics. The only thing that stops validation outright is a file that can't be read (section 2.2).

A project is valid if it has no errors. Warnings are allowed, but see section 4.4 before reading too much into a clean result.

| Rule                             | Level   | When                                                           |
| -------------------------------- | ------- | -------------------------------------------------------------- |
| `project-website-iri`            | error   | `website` isn't an IRI with a scheme (RFC 3987)                |
| `project-usage-iri`              | error   | a usage `resource` isn't an IRI with a scheme                  |
| `meta-missing`                   | warning | there's no `.meta.json`                                        |
| `meta-created-format`            | error   | `created` isn't an RFC 3339 `date-time`                        |
| `meta-metamodel-iri`             | error   | `metamodel` isn't an IRI with a scheme                         |
| `index-path-syntax`              | error   | an `index` path fails section 4.2                              |
| `index-path-boundary`            | error   | an `index` path goes through a link or into a nested project   |
| `index-file-missing`             | error   | an `index` path doesn't lead to a regular file                 |
| `index-unreferenced-file`        | warning | a model file from section 3.3 isn't in `index`                 |
| `checksum-path-syntax`           | error   | a `checksum` path fails section 4.2                            |
| `checksum-path-boundary`         | error   | a `checksum` path goes through a link or into a nested project |
| `checksum-file-missing`          | error   | a `checksum` path doesn't lead to a regular file               |
| `checksum-algorithm-invalid`     | error   | the algorithm isn't in the Table 13 list                       |
| `checksum-algorithm-unsupported` | warning | the algorithm is allowed but syspm can't compute it            |
| `checksum-value-invalid`         | error   | the value isn't hex of the right length                        |
| `checksum-mismatch`              | error   | the file doesn't match its checksum                            |
| `file-unreadable`                | error   | a file or directory that had to be read couldn't be            |

For the three IRI rules, a value must be a full RFC 3987 IRI that starts with a scheme, such as `https:`. Relative references are rejected. A fragment is allowed, so this is looser than RFC 3987's `absolute-IRI`.

The warnings are warnings because the standard makes those things optional. `.meta.json` is optional and so is listing every file in `index`. Table 13 also says checksums may cover "some or all" files, so a file without one isn't reported.

For `created`, the whole RFC 3339 `date-time` grammar applies. The date has to exist, the separator has to be `T` (a space isn't allowed), an offset is required, and a leap second is only valid at 23:59:60 UTC on the last day of a month. Looser ISO 8601 forms are rejected.

### 4.2 Recorded paths

syspm didn't write the paths in `index` and `checksum`, so it doesn't trust them. A path is rejected if it is empty or absolute, contains a backslash or NUL, starts with a drive letter like `C:`, has an empty, `.` or `..` segment, doesn't end in a file name with an extension, or is `.project.json` or `.meta.json` (neither is a model interchange file).

`index` paths are checked against what's on disk. They don't have to appear in the section 3.3 listing, so JSON and XMI model files are fine in `index`.

### 4.3 Following a recorded path

A path that passes section 4.2 is resolved one segment at a time, without following links. Hitting a symbolic link, or entering a directory that has its own `.project.json`, is a boundary error. Hidden directories are allowed here, because naming one in metadata is a deliberate choice. Only regular files are opened.

There's a gap between checking and opening. These checks guard against what's in the project, and don't stop another process from changing the project mid-validation.

On a case-insensitive file system an `index` path whose case doesn't match the file name still finds the file. The file is then also flagged by `index-unreferenced-file`.

### 4.4 Checksums

Values are hex. The standard doesn't actually say so, but every OMG file uses hex. They're decoded before comparison, so case doesn't matter, and a value that won't decode is `checksum-value-invalid` (not a mismatch). The checksum is over the file's raw bytes.

Table 13 allows these algorithm names, spelled exactly this way: `SHA1`, `SHA224`, `SHA256`, `SHA-384`, `SHA3-256`, `SHA3-384`, `SHA3-512`, `BLAKE2b-256`, `BLAKE2b-384`, `BLAKE2b-512`, `BLAKE3`, `MD2`, `MD4`, `MD5`, `MD6`, `ADLER32`. Anything else is `checksum-algorithm-invalid`.

syspm only computes `SHA256` at the moment. Any other allowed algorithm gets `checksum-algorithm-unsupported`, meaning the file wasn't checked. So no errors doesn't mean every file was verified. Anything that needs verification should treat that warning as a failure. Adding an algorithm later just moves it out of the warning.

Missing, unreadable and changed files are always errors.

### 4.5 What validation doesn't do

This isn't a compliance checker. It doesn't check that `index` names match the elements declared in each file (that needs a model parser), and it doesn't check full conformance to the schema's structure. Rules that only make sense for archives, like complete `index` and `checksum` coverage, will come with archive support.

## 5. Creating a project

`init` writes a `.project.json` into a directory. It doesn't create `.meta.json` or any model files.

The file contains `name` and `version`, pretty-printed, with a trailing newline. The name defaults to the directory's name, resolving paths like `.` first. It's an error if the name is empty, or if there's no usable default because the directory has no name or its name isn't valid UTF-8. The version defaults to `0.1.0`.

If anything called `.project.json` already exists, even a broken link, `init` fails and leaves it untouched. The file is created in one step that fails if the name is taken, so nothing can sneak in between the check and the write.

A project further up the tree doesn't stop `init`. The new one is just a separate project (section 3.2).

A fresh project validates with a single warning, `meta-missing`.
