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
- Small, focused changes are easiest to review and merge.
- If your change touches non-trivial behavior, add or update tests.
- If your change introduces new concepts or APIs, update the relevant READMEs and docs.
- Follow the relevant justfiles and READMEs for test coverage

## Code Style

Before opening a PR, run `just precommit` from the repository root.
This runs the same blocking gates that CI runs for language, library, service, app, and bridge.
Use `just fmt` for formatting, `just check` for broad checks, and `just test` for the full local test matrix.
See [TESTING.md](TESTING.md) for the full test matrix and suite details.

## Commit Style

Use conventional commits for all repository changes.
- Follow `type(scope): summary` with an imperative summary and keep it under 100 characters.
- Use one of `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `dev`, `ci` as the type.
- For example: `feat(language): improve error span precision (to sub-token granularity)`.
- Do not mention non-human authors or contributors in commit messages. We don't care.

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

## Release

Release CI is tag driven and runs on `v*` pushes.
The release workflow fails if the pushed tag does not match `VERSION.txt`, and it also fails when tracked version files drift or `CHANGELOG.md` has no section for `VERSION.txt`.

### Procedure

Use the top level `just` recipes so versioning and changelog automation stay consistent:
 - `just bump patch`, `just bump minor`, or `just bump major` to update all tracked version files.
 - `just release-changelog` to generate the `CHANGELOG.md` section for the current version.
 - `just release-changelog` also refreshes `bridge/dart/CHANGELOG.md` and syncs `bridge/dart/LICENSE` from `LICENSE.txt`.
 - `just release-validate` to verify tag, tracked versions, and changelog state for the current version.
 - `just release patch` to bump, validate, update changelog, commit, and tag in one command.
 - `just release-push patch` to do the same flow and push `main` plus the release tag.
(This only works if you have the keys, so either locally with `.env.local` or via CI.)

### Changelog

Destack is alpha software, so release entries do not need migration notes yet.
 - Keep the root changelog concise and user facing, and avoid dumping every internal commit line.
 - Only include items with clear external impact for users, operators, or package consumers.
 - Treat `CHANGELOG.md` as the canonical monorepo changelog for release history.
 - Treat package local changelogs as thin package metadata, and keep them short with a pointer to the root changelog.
Use `just release-changelog` as the single source of truth for changelog updates during release preparation.
The changelog is generated automatically by that command, and manual edits are optional curation before release tagging.

### Credentials

Publishing commands load credentials from `.env.local` via `just`.
Use the following variable level matrix for the GitHub Actions `release` environment.

| Key | Kind | Required when | Purpose |
|--------------|----------|------------------------------------------------------|------------------------------------------------------------|
| `CARGO_TOKEN` | Secret | Always | crates.io publishing |
| `NUGET_PUBLISH_USERNAME` | Variable | Always | NuGet trusted publishing identity |
| `MAVEN_REPOSITORY_USERNAME` | Secret | Always | Maven Central portal username |
| `MAVEN_REPOSITORY_PASSWORD` | Secret | Always | Maven Central portal password |
| `MAVEN_GPG_PRIVATE_KEY` | Secret | Always | Armored private key for Maven signing |
| `MAVEN_GPG_PASSPHRASE` | Secret | Always | Passphrase for Maven signing key |
| `MAVEN_GPG_KEY_ID` | Variable | Always | Key id used by Maven GPG plugin |
| `RELEASE_GPG_PRIVATE_KEY` | Secret | Always | Armored private key for release artifact signatures |
| `RELEASE_GPG_PASSPHRASE` | Secret | Always | Passphrase for release artifact signing key |
| `RELEASE_GPG_KEY_ID` | Variable | Always | Key id used for release artifact signatures |
| `RUBYGEMS_OIDC_ROLE` | Variable | Always | RubyGems trusted publishing role |
| `HEX_API_KEY` | Secret | Always | Hex publishing |
| `VSCE_PAT` | Secret | Always | VS Code extension publishing |
| `RELEASE_PUBLISH_ZED` | Variable | Optional | Enables zed registry publish on release tags |
| `ZED_GITHUB_TOKEN` | Secret | `RELEASE_PUBLISH_ZED == true` | GitHub token for zed registry PR lane |
| `ZED_REGISTRY_PUSH_TO` | Variable | `RELEASE_PUBLISH_ZED == true` | zed registry target fork/owner |

For local live publishing outside CI, token based env vars such as `NPM_TOKEN`, `CARGO_TOKEN`, `PYPI_TOKEN`, and `VSCE_PAT` are still supported.
The release workflow uses trusted publishing or OIDC wherever possible.

### Integrity

Release artifacts include `manifest.json` and `SHA256SUMS`.
CI produces detached armored signatures for both files with the dedicated `RELEASE_GPG_*` key.
The release lane also signs installer scripts as `install.sh.asc` and `install.ps1.asc`.
The public verification key is published as `app/cli/install/release-signing-public.asc` and attached to GitHub releases.
