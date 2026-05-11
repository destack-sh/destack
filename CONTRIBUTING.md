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

Before committing, run `just quick` from the repository root.
Before pushing or landing a non-trivial change, run `just full` from the repository root at least once.
`just quick` is the normal local confidence gate.
`just full` is the deepest local verification sweep and should be the final pre-push gate for broad, risky, or cross-cutting changes.
Use `just fmt` for formatting and `just check` for static checks when you are iterating on one area.
See [TESTING.md](TESTING.md) for the full test matrix and suite details.

## Versioning And Status

Destack uses one canonical monorepo release version from [VERSION.txt](VERSION.txt).
The canonical release history lives in GitHub releases.
Do not bump versions during normal development.
Only bump versions through `just bump` or `just release`.

Public project inventories and maturity live in the area README tables.

## Commit Style

Use conventional commits for all repository changes.
- Follow `type(scope): summary` with an imperative summary and keep it under 100 characters.
- Use one of `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `dev`, `ci` as the type.
- Use the full scope (sometimes stylisied) like `language/ast`, `language/compiler/analyze`, `library/ui`, ...
- If the commit touches multiple scopes either use the highest most, use `all`, or (if large enough) break into multiple smaller commits
- For example: `feat(language/source): improve error span precision (to sub-token granularity)`.
- In case of doubt, look at the past 50 or so commit messages for common style.
- Do not mention non-human authors or contributors in commit messages. We don't care.

## Security

If you find a security issue, please follow [SECURITY.md](SECURITY.md).

## Licensing

Repository authored code is MIT unless otherwise noted.
Vendored components may retain their own licenses.
See [LICENSE.txt](LICENSE.txt) and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Setup

Destack is *developed* primarily using Rust and TypeScript (and Destack itself, of course).
To contribute to Destack and build it yourself locally you will need at least `cargo`, `bun`, and `just`:

- [Rust](https://rustup.rs/): Rust compiler (`nightly-2025-11-27`, see [rust-toolchain.toml](rust-toolchain.toml))
- [Bun](https://bun.sh/): JavaScript runtime and package management
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

We use `justfile`s as the source of truth for all commands:
```sh
just install        # setup everything
just check          # run repository static checks
just test           # run area test aggregates
just fmt            # format all code
just quick          # run the repository quick gate
just full           # run the repository full gate
just clean          # clean all build artifacts
just publish        # publish all packages
```

If you are working in one area only, use scoped area gates:
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

Release CI is tag driven and runs on `v*` pushes:
 - Use `just release` to prepare a local patch release commit and tag.
 - Use `just release minor` or `just release major` when you want a non-default bump.
 - Use `just release-push` to push the current release commit and tag.
 - Nightly is the high-frequency canary channel.
See [RELEASE.md](RELEASE.md) for the canonical release runbook, credential matrix, signing model, and failure recovery guidance.
