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
To contribute to Destack and build it yourself locally you will need at least `cargo`, `bun`, and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2026-05-26`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

We use `justfile`s as the source of truth for all commands. 
See the relevant directories we're working on for the relevant just recipes.

## Release

The root [destack.json](destack.json) declares the complete distribution and its `YEAR.MONTH.MICRO` version.
Packages and products declare their own stability.
Do not edit versions directly: use `just next-version`, `just set-version`, or `just release`.

Prepare and publish a release from the repository root:

```sh
just check-quick
just check-full
just release
# review the release commit and vYEAR.MONTH.MICRO tag
just release-push
```

Pushing the tag runs [.github/workflows/release.yml](.github/workflows/release.yml).
Use `just validate-release` and `just publish --dry-run` for additional packaging checks.
CI credentials live in the GitHub `release` environment, while local publishing reads `.env.local`.
