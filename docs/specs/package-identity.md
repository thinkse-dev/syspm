# Package identity

This covers what identifies a package, how versions and constraints are read, and which data syspm owns. It is based on section 10.3 of KerML 1.0 and the normative schema, `KerML-Model-Interchange.json`. How the two standard files are found, read and validated is in [project-metadata.md](project-metadata.md).

Section numbers below won't be renumbered, so tests and other docs can cite them.

## Background

The language target is SysML 2.1 and KerML 1.1. The KerML 1.1 Beta 1 document, published with the `2026-05` release of `Systems-Modeling/SysML-v2-Release` (commit `de1070ae8e79c21532b8004fc663d47b35d0e9fa`), defines the same section 10.3 properties as 1.0 (Tables 12 and 13) and marks no 1.1 change in clause 10. OMG publishes no separate 1.1 interchange schema.

What the standard gives us:

- A project has a `name` and a `version`. Each usage has a `resource` IRI and an optional `versionConstraint`. A project declares no identifier for itself.
- Section 10.3 recommends, but doesn't require, semantic versions for projects and semantic version ranges for constraints. It doesn't define a range grammar. It points to an existing one as an example. The schema types `versionConstraint` only as a string.
- If a constraint is given, only versions that meet it may be used, and tools that support such version formats should report violations.
- The schema doesn't set `additionalProperties: false`, so it allows properties it doesn't define.

What the published files show:

- The ten standard library projects from that release carry identical `.project.json` files in their directories and in their `.kpar` archives. They name their dependencies by usage IRIs such as `https://www.omg.org/spec/KerML/20250201/Semantic-Library.kpar`, the published location of the KerML 1.0 archive, with the constraint `1.1.0-dev.20260501`. The used project's own files never state that IRI. Its `.project.json` only says `"name": "Kernel Semantic Library"`.
- Every KerML library declares the `metamodel` `https://www.omg.org/spec/KerML/20250201`, even at version `1.1.0-dev.20260501`.
- An archive written by another tool, in `tests/fixtures/toolkit-archive`, takes its project name from the output file name and always has version `1.0.0`.

## 1. Who owns which data

The standard files are the package definition. syspm reads them and adds nothing to them (project-metadata.md section 1).

Everything else that describes a package or how it resolves, and that other tools or people need to reproduce a result, is shared resolution data. That means the shared source bindings and the lock. Each such format is documented, versioned, readable with general-purpose tools, and not tied to syspm by name or content. Each gets its own spec, which also gives the file name.

`syspm.toml` holds only syspm settings: how syspm behaves, and machine-local preferences such as the cache location, output, and a local path standing in for a source (section 5).

## 2. Logical identity

A package's logical identity is the IRI that a using project writes in `usage.resource`.

IRIs are compared as exact strings. Two IRIs that differ in any character are different packages. No normalization is applied for now.

A project's own files don't contain that IRI. So a **source binding** says which project an IRI denotes (section 5). syspm never works it out from `name`, a file name or a model namespace.

`name` is a label. It never identifies, merges or matches packages. Two projects with the same `name` and different IRIs are different packages. One IRI bound to projects with different names is still one package, and the difference is reported.

An IRI is ambiguous when two or more candidate projects with the same version and different content could be bound to it. That's an error. syspm doesn't choose.

## 3. Instance identity

A package instance is identified by its logical IRI, its `version` and a content digest. The digest will be specified with the included tree.

The origin, such as a path or a repository with a subdirectory and a commit, is recorded separately and doesn't change identity.

Two origins for the same IRI and version are the same instance only when their content digests are equal, as with a mirror. Otherwise the IRI is ambiguous (section 2).

Only the digest can detect content that changed while the name and version stayed the same.

## 4. Versions and constraints

A project's `version` is a required string, and syspm treats it as opaque. It isn't checked.

For now syspm supports a deliberately small part of what section 10.3 recommends. A `versionConstraint` is an **exact constraint** when it is a complete SemVer 2.0.0 version with no build metadata: `MAJOR.MINOR.PATCH`, optionally followed by a prerelease such as `-dev.20260501`, with nothing before or after it. It is met only by a bound project whose `version` is the identical string.

Any other constraint is reported as unsupported, not invalid, and syspm doesn't guess what it means. That includes:

- a range, such as `^1.0.0`
- a partial version, such as `1.0` or `1`
- a prefixed version, such as `=1.0.0` or `v1.0.0`
- a version with build metadata, such as `1.0.0+a`
- leading or trailing whitespace
- a string that isn't a SemVer version, such as `2024a`

A usage with no constraint accepts any bound version, and the result records it as unconstrained. Section 10.3 only restricts versions when a constraint is given.

Why exact means a complete version without build metadata: in the range grammar section 10.3 points to, a partial version such as `1.0` is a range (`>=1.0.0 <1.1.0-0`), and build metadata is ignored, so `1.0.0+a` also matches `1.0.0+b`. Reading either as an exact literal now would change its meaning once ranges are added. A complete version without build metadata means the same in both. A version that differs only in build metadata, such as `1.0.0+b` against `1.0.0`, fails here where that grammar would accept it. That's stricter, never looser.

Ranges can be added later without breaking exact constraints. Every version an exact constraint accepts is also accepted under that range grammar, so adding ranges can only accept more. Adding them means picking a precise grammar, since the standard only gives an example, and deciding how prerelease versions are ordered.

## 5. Source bindings and overrides

A binding maps an IRI to an origin. It never changes a constraint. If the bound project's version doesn't meet the constraint, that's an error, not a silent override.

Bindings shared with a project or workspace are shared resolution data (section 1). A binding that refers to a machine-local absolute path is local configuration and is never written into shared data.

An override is a separate, explicit record that accepts a version the constraint rejects, and it gives a reason. It is always reported, and a binding never implies one. Its format is specified with the binding format.

## 6. Exported namespaces

A project's declared exported namespaces are the keys of its `.meta.json` `index`, which Table 13 defines as the names in the project's global scope.

Before a modeling tool loads a closure of packages, these keys are compared across it to find names that more than one package provides.

A project without `.meta.json` (a `meta-missing` warning, project-metadata.md section 4.1) has unknown exports. The comparison reports it as **not checked**, never as providing nothing.

Whether `index` matches the elements the files declare needs a model parser, so it's left to the modeling tool (project-metadata.md section 4.5).

## 7. Language and standard-library requirements

`metamodel` is informational. It's a declaration, and it isn't enough to check a language version: the standard library declares the KerML 1.0 metamodel IRI at version `1.1.0-dev`.

A project's requirement on the standard library is the set of its usages whose IRIs denote standard library projects. They're checked against the identities and versions that the selected standard-library provider set declares. A constraint that the selected version doesn't meet fails before any modeling tool runs.

How a provider set declares which IRIs it provides is part of that set's tool-neutral description.

## 8. Properties the schema doesn't define

syspm keeps them when it reads a file, as JSON values, so it can report them. A name repeated in any object, at any depth, is a read failure, the same as for every other property (project-metadata.md section 2.2).

The values read aren't lossless. Property order isn't recorded, and a number is kept as a 64-bit integer or a double, so an integer such as `18446744073709551617` or a very precise decimal comes back changed. These values are for reporting only and must not be used to write a file.

syspm still writes no such property into the standard files.

When syspm has to change a standard file, it keeps every property it doesn't change, these included, unchanged and in their original order. Files it doesn't need to change stay byte for byte the same. syspm doesn't rewrite existing files yet. The first command that does must first choose a lossless representation, and test a round trip of `tests/fixtures/additional-properties` that covers nested repeated names, numbers outside the 64-bit and double ranges, property order, and properties left unchanged.

## 9. Specification baseline

Validation follows KerML 1.0 section 10.3 and the 1.0 schema, which KerML 1.1 Beta 1 leaves unchanged. This is revisited when OMG publishes a 1.1 interchange schema or changes section 10.3.

## Open questions

- Whether IRIs need normalization.
- Which range grammar to support, and how prerelease versions are ordered. That only matters once there are ranges.
- If a later KerML version adds an identifier that a project declares for itself, section 2 should be revisited.
