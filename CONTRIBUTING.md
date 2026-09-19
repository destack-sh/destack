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
- [Deno](https://deno.com/): JavaScript runtime and package management (2.9.7)
- [just](https://github.com/casey/just): Scripts and command runner
- [Python](https://python.org/): Project docs validation and codegen utilities

## Commands

We use `justfile`s as the source of truth for all commands. 
See the relevant directories we're working on for the relevant just recipes.

## Platform

Select one infrastructure deployment explicitly:

```sh
just platform/stack/check
just platform/stack/diff shared
just platform/stack/diff production eu
just platform/stack/deploy production eu
```

- `shared` manages domains, website routing, downloads, and state storage.
- `development` and `production` each select `global`, `eu`, or `us`.
- `platform/stack/deployment/` contains the deployment variables.
- `deploy` displays a saved plan and requires confirmation before applying it.
- `just platform/deploy` publishes the website; infrastructure deployment is separate.
- PlanetScale databases are disabled; enabling them requires an organization and service credentials.
- Existing EU package buckets move from shared state through non-destructive removal and regional import blocks.
- `package_import_id` selects an existing bucket to adopt; omit it when creating a bucket.
- Saved plans use private per-run directories under `platform/stack/.terraform/` and may contain secrets.
- Freeze infrastructure deployments and back up state before applying the shared plan followed by both EU regional plans.

## Release

The root [package.json](package.json) declares the repository’s `YEAR.MONTH.MICRO` version.
[Release configuration](dev/release/config.json) declares the distribution’s stability.
Check, build, and pack the CLI and desktop from the repository root:

```sh
just check
just dev/release/build
just dev/release/pack
```

- [The release workflow](.github/workflows/release.yml) checks matching pull requests and pushes to main.
- [The update workflow](.github/workflows/update.yml) renews signed download metadata daily; it does not build a release.
- Enable `publish` on a manual run from `main` to sign and publish downloads to `download.destack.sh`.
