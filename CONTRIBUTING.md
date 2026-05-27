# Contributing

We welcome serious non-slop bug reports, fixes, and feature suggestions and such: join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

Destack is not generally open for public contributions at this point as we are in very active development with a singular focus - a fully integrated software stack for optimal, correct, integrated software.

## Checklist

- Before committing, run `just check` or `just check-quick` from the repository root.
- Before pushing or landing a non-trivial change, run `just check-full` from the repository root at least once.
- `just check-quick` is the normal local confidence check.
- `just check-full` is the deepest local validation sweep and should be the final pre-push check for broad, risky, or cross-cutting changes.
- Use `just fmt` for formatting and `just lint` for static checks when you are iterating on one area.
- See [TESTING.md](TESTING.md) for the test matrix and suite details.

## Versioning

Destack uses one canonical monorepo release version from [VERSION.txt](VERSION.txt).
Do not bump versions during normal development.
Only bump versions through `just bump` or `just release`.

## Commits

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

We use `justfile`s as the source of truth for all commands:
```sh
just install        # setup everything
just lint           # run repository static checks
just test           # run area test aggregates
just fmt            # format all code
just check          # run the normal repository check
just check-quick    # run the explicit normal repository check
just check-full     # run the repository check with slow suites
just clean          # clean all build artifacts
just publish        # publish all packages
```

If you are working in one area only, use scoped area checks:
```sh
just language/check-quick
just language/check-full
just library/check-quick
just library/check-full
just service/check-quick
just service/check-full
just app/check-quick
just app/check-full
just bridge/check-quick
just bridge/check-full
```

## Release

Release CI is tag driven and runs on `v*` pushes:
 - Use `just release` to prepare a local patch release commit and tag.
 - Use `just release minor` or `just release major` when you want a non-default bump.
 - Use `just release-push` to push the current release commit and tag.
 - Nightly is the high-frequency early-access channel.
 - Canary is reserved for internal validation builds.
See [RELEASE.md](RELEASE.md) for the canonical release runbook, credential matrix, signing model, and failure recovery guidance.
