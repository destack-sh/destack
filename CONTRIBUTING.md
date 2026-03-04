# Contributing

> **You don't need to do any of this to *use* Destack!** Go to [Getting Started](README.md#getting-started) to install and run Destack.

We welcome bug reports, fixes, and feature suggestions and any other ideas.
Join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

PRs for small fixes are probably fine, but larger unsolicited PRs are unlikely to be accepted - the whole point of the project is tight integration.
Destack is in very active development with a singular focus: a fully integrated software stack for optimal, correct, integrated software.

## Before You Start

If you want to make a non trivial change, please open an issue or start a Discord thread first.
We move fast and make cohesive design decisions, so early alignment saves everyone time.

## What We Expect

We value clarity, correctness, and performance.
Small, focused changes are easiest to review and merge.
If you throw AI slop at us, we'll throw AI slop right back and tell your mom.

If your change touches behavior, add or update tests.
If your change introduces new concepts or APIs, update the relevant READMEs and docs.

## Code Style

Before opening a PR, run `just precommit` from the repository root.
This runs the same blocking gates that CI runs for language, library, service, app, and bridge.
Use `just fmt` for formatting, `just check` for broad checks, and `just test` for the full local test matrix.
See [TESTING.md](TESTING.md) for the full test matrix and suite details.

## Security

If you find a security issue, please follow [SECURITY.md](SECURITY.md).

## Structure

This is the open source monorepo containing the language, library, service, app, bridge, and template layers:

| Directory | Description | README |
|--------------|----------------------------------------------------------------|-------------------------------|
| `language/`  | Language toolchain (parser, compiler, formatter, LSP, etc.)    | [language/README](language/README.md)  |
| `library/`   | Standard library (entity, telemetry, math, physics, UI, etc.)  | [library/README](library/README.md)   |
| `service/`   | Runtime and developer services (daemon, lsp, lsp-server, lsp-types) | [service/README](service/README.md) |
| `app/`       | First-party applications and operator tools (cli, future apps) | [app/README](app/README.md) |
| `bridge/`    | SDKs and external integrations (napi, wasm, rust, python, vscode, zed, bun, vite) | [bridge/README](bridge/README.md) |
| `template/`  | Project templates and initializer package | [template/README](template/README.md) |
| `docs/`      | Additional project documentation                                  | [docs/README](docs/README.md)          |

## Setup

Destack is *developed* primarily using Rust and TypeScript (and Destack itself, of course).
To contribute to Destack and build it yourself locally you will need at least `cargo`, `bun`, and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Scripts and codegen utilities (*optional*)

## Commands

We use `justfile`s as the source of truth for all commands:
```sh
just precommit      # canonical pre-PR gate, mirrors blocking CI
just ci             # blocking CI gates only
just install        # setup everything
just check          # check & lint everything
just fmt            # format all code
just test           # run all tests
just bench          # run all benchmarks
just fuzz           # run all fuzzers
just clean          # clean all build artifacts
just publish        # publish all packages
```

If you are working in one area only, use scoped gates:
```sh
just language/ci
just library/ci
just service/ci
just app/ci
just bridge/ci
```

## Release Credentials

Publishing commands load credentials from `.env.local` via `just`.
Use the following variable level matrix for the GitHub Actions `release` environment.

| Key | Kind | Required when | Purpose |
|--------------|----------|------------------------------------------------------|------------------------------------------------------------|
| `NPM_USE_TRUSTED_PUBLISHING` | Variable | Optional | Enables npm trusted publishing in CI |
| `NPM_TOKEN` | Secret | `NPM_USE_TRUSTED_PUBLISHING != true` | Fallback npm authentication |
| `CARGO_TOKEN` | Secret | Always | crates.io publishing |
| `NUGET_USE_TRUSTED_PUBLISHING` | Variable | Optional | Enables NuGet trusted publishing in CI |
| `NUGET_PUBLISH_USERNAME` | Variable | `NUGET_USE_TRUSTED_PUBLISHING == true` | NuGet trusted publishing identity |
| `NUGET_API_KEY` | Secret | `NUGET_USE_TRUSTED_PUBLISHING != true` | Fallback NuGet authentication |
| `MAVEN_REPOSITORY_USERNAME` | Secret | Always | Maven Central portal username |
| `MAVEN_REPOSITORY_PASSWORD` | Secret | Always | Maven Central portal password |
| `MAVEN_GPG_PRIVATE_KEY` | Secret | Always | Armored private key for Maven signing |
| `MAVEN_GPG_PASSPHRASE` | Secret | Always | Passphrase for Maven signing key |
| `MAVEN_GPG_KEY_ID` | Variable | Always | Key id used by Maven GPG plugin |
| `RUBYGEMS_USE_TRUSTED_PUBLISHING` | Variable | Optional | Enables RubyGems trusted publishing in CI |
| `RUBYGEMS_OIDC_ROLE` | Variable | `RUBYGEMS_USE_TRUSTED_PUBLISHING == true` | RubyGems trusted publishing role |
| `RUBYGEMS_API_KEY` | Secret | `RUBYGEMS_USE_TRUSTED_PUBLISHING != true` | Fallback RubyGems authentication |
| `HEX_API_KEY` | Secret | Always | Hex publishing |
| `PUB_DEV_USE_OIDC` | Variable | Optional | Enables pub.dev trusted publishing in CI |
| `PUB_DEV_CREDENTIALS_JSON` | Secret | `PUB_DEV_USE_OIDC != true` | Fallback pub.dev credentials |
| `VSCE_PAT` | Secret | Always | VS Code extension publishing |
| `ZED_GITHUB_TOKEN` | Secret | `publish_zed == true` | GitHub token for zed registry PR lane |
| `ZED_REGISTRY_PUSH_TO` | Variable | `publish_zed == true` | zed registry target fork/owner |

For local live publishing outside CI, token based env vars such as `NPM_TOKEN`, `CARGO_TOKEN`, `PYPI_TOKEN`, and `VSCE_PAT` are still supported.
