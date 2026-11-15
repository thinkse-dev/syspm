# syspm

**Status: early development. Not yet a functional package manager.**

`syspm` is a standalone, open-source package manager for SysML v2 and KerML
model projects, focused on verifiable, reproducible model baselines.

## Build

Requires [rustup](https://rustup.rs/); the toolchain is pinned in
`rust-toolchain.toml`.

```bash
cargo build --workspace
cargo run -p syspm -- --help
./scripts/check.sh   # run the same checks as CI
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Licensing

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
