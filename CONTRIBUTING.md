# Contributing

We welcome serious non-slop bug reports, issues and feature suggestions: join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

Destack is not generally open for public contributions at this point.

## Versioning

The root [destack.json](destack.json) declares the complete Destack distribution and workspace.
Its `version` uses `YEAR.MONTH.MICRO` and advances weekly, while each Package or Product declares its own Stability.
Do not change versions during normal development, only advance versions through `just next-version` or `just release`.

## Security

If you find a security issue, please follow [SECURITY.md](SECURITY.md).

## Licensing

Repository authored code is MIT unless otherwise noted; vendored components may retain their own licenses.
See [LICENSE.txt](LICENSE.txt) and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Setup

Destack is *developed* primarily using Rust and TypeScript (and Destack itself, of course).
To contribute to Destack and build it yourself locally you will need at least `cargo`, `bun`, and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2026-05-26`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

We use `justfile`s as the source of truth for all commands. 
See the relevant directories we're working on for the relevant just recipes.

## Release

Release CI is tag driven and runs on `v*` pushes:
 - Use `just release` to prepare the next weekly release commit and tag.
 - Use `just release-push` to push the current release commit and tag.
 - Nightly is the high-frequency early-access channel.
 - Canary is reserved for internal validation builds.
See [RELEASE.md](RELEASE.md) for the canonical release runbook, credential matrix, signing model, and failure recovery guidance.
