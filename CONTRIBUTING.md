# Contributing

We welcome serious non-slop bug reports, issues and feature suggestions: join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

Destack is not generally open for public contributions at this point.

## Security

If you find a security issue, please follow [SECURITY.md](SECURITY.md).

## Licensing

Repository authored code is MIT unless otherwise noted; vendored components may retain their own licenses.
See [LICENSE.txt](LICENSE.txt) and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Setup

Destack is *developed* primarily using Rust and TypeScript (and Destack itself, of course).
Install the tools needed by the packages you are working on:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2026-05-26`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management (1.4.2)
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

We use `justfile`s as the source of truth for most commands. 
See the relevant directories we're working on for the relevant just recipes.
