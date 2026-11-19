# Contributing

We welcome serious non-slop bug reports, issues and feature suggestions: join our [Discord](https://discord.gg/xUFQ45TWYd) to chat and discuss.

Destack is not generally open for public contributions at this point.

## Security

If you find a security issue, please follow [SECURITY.md](SECURITY.md).

## Licensing

The monorepo is Apache 2.0 licensed unless otherwise noted; vendored components may retain their own licenses.
See [LICENSE.txt](LICENSE.txt).

## Setup

Destack is *developed* primarily using TypeScript (and Destack itself, of course).
Install the tools needed by the packages you are working on:

- [Rust](https://rustup.rs/): Rust compiler (stable), only for the native desktop and release builds
- [Bun](https://bun.sh/): JavaScript runtime and package management (1.4.2)
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

The root `justfile` runs the repository's commands.
Packages have no justfiles of their own: `destack check`, `destack fix` and `destack test` check, format and test one package, and the recipes below run them with the checkout's sources.

```sh
just check-package @destack/ui    # check one package: format, types and lint
just test @destack/ui             # test one package, or its unit tier alone with --fast
just check-fast                   # check and test the packages a change affects, as CI does
just fix                          # format and fix the repository
just local serve                  # serve the local universe
cd @platform/site && bun run dev  # serve the site at http://localhost:3737
```
