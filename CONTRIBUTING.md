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

Before opening a PR, run `just quick` from the repository root.
This runs the same quick gate that CI enforces across language, library, service, app, and bridge.
Use `just quick` for fast local confidence, `just fmt` for formatting, `just check` for static checks, and `just full` for the deepest local verification sweep.
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
just install        # setup everything
just check          # run static checks
just test           # run scoped test suites
just fmt            # format all code
just quick          # run the fast repository gate
just full           # run the full repository gate
just clean          # clean all build artifacts
just publish        # publish all packages
```

If you are working in one area only, use scoped gates:
```sh
just language/quick
just language/full
just library/quick
just library/full
just service/quick
just service/full
just app/quick
just app/full
just bridge/quick
just bridge/full
```

## Release

Release CI is tag driven and runs on `v*` pushes.
Use `just release patch` to prepare a local release commit and tag.
Use `just push-release` to push the current release commit and tag.
See [RELEASE.md](RELEASE.md) for the canonical release runbook, credential matrix, signing model, and failure recovery guidance.
