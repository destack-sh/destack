# Release

Destack releases are tag driven.
The canonical release tag format is `vX.Y.Z`.
Release CI reuses the repository `full` verification depth and then adds artifact packaging, signatures, installer checks, updater checks, and publishing.

## Model

Use the repository root `just` recipes as the public operator surface.
Do not bypass them with ad hoc script invocations unless you are repairing a failed release.

| Command | Purpose |
|---------|---------|
| `just bump patch`, `just bump minor`, `just bump major` | Update `VERSION.txt` and all tracked version files |
| `just generate-release-changelog` | Refresh the root changelog entry for the current version |
| `just validate-release` | Validate tag, tracked versions, and changelog state |
| `just release patch` | Bump, validate, generate changelog, commit, and tag locally |
| `just push-release` | Push the current release commit and local release tag |
| `just publish --dry-run` | Dry run the full multi-registry publish fanout |
| `just publish-release` | Publish live using the normal staged release path |
| `just publish-release-local` | Publish live using local CLI binary staging |

## Standard Flow

This is the normal operator path.

1. Run `just quick`.
2. Run `just full` if you want local deep validation before tagging.
3. Run `just release patch`, `just release minor`, or `just release major`.
4. Review the resulting commit, tag, and changelog.
5. Run `just push-release`.

Pushing the `vX.Y.Z` tag triggers [.github/workflows/release.yml](/Users/florian/symbol/destack/.github/workflows/release.yml).

## Local Validation

Run these commands from the repository root when preparing a release manually.

```sh
just quick
just full
just generate-release-changelog
just validate-release
just publish --dry-run
```

Use `just publish --dry-run` when you want a broad packaging and registry preflight without performing a live publish.

## Local Live Publish

There are two supported local live publish paths.

### Standard

Use this when release artifacts have already been prepared by CI or by a previous local packaging step.

```sh
just publish-release
```

### Local CLI Staging

Use this when you need to stage CLI binaries locally before publish.

```sh
just publish-release-local
```

`publish-release-local` prefers `release-cli-assets` or `DESTACK_CLI_ARTIFACTS` when present.
Otherwise it builds and stages CLI binaries for the configured `DESTACK_RELEASE_TARGETS`, or the host target when that fallback is supported.

## CI Release Flow

The release workflow is tag driven and intentionally not `workflow_dispatch` driven.
It validates release metadata first, then runs scoped `full` gates, runtime reusable workflows, artifact packaging, signature generation, installer verification, and publishing.

Important workflow pieces:

- [release.yml](/Users/florian/symbol/destack/.github/workflows/release.yml)

## Changelog

Destack is alpha software, so release entries do not need migration notes yet.
Keep the root changelog concise and user facing.
Only include items with clear external impact for users, operators, or package consumers.
Treat [CHANGELOG.md](/Users/florian/symbol/destack/CHANGELOG.md) as the canonical monorepo release history.
Treat package local changelogs as thin package metadata that can point back to the root changelog.

`just generate-release-changelog` is the single source of truth for release changelog generation.
That command also refreshes `bridge/dart/CHANGELOG.md` and syncs `bridge/dart/LICENSE` from `LICENSE.txt`.

## Release Integrity

Release artifacts include `manifest.json` and `SHA256SUMS`.
CI produces detached armored signatures for both files with the dedicated `RELEASE_GPG_*` key.
The release lane also signs `install.sh` and `install.ps1` as detached `.asc` files.
The public verification key is published as [release-signing-public.asc](/Users/florian/symbol/destack/app/cli/install/release-signing-public.asc) and attached to GitHub releases.

## Credentials

Publishing commands load credentials from `.env.local` via `just` when run locally.
GitHub Actions uses the `release` environment for the CI path.

| Key | Kind | Required when | Purpose |
|-----|------|---------------|---------|
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
| `ZED_GITHUB_TOKEN` | Secret | `RELEASE_PUBLISH_ZED == true` | GitHub token for the zed registry PR lane |
| `ZED_REGISTRY_PUSH_TO` | Variable | `RELEASE_PUBLISH_ZED == true` | zed registry target fork or owner |

For local live publishing outside CI, token-based variables such as `NPM_TOKEN`, `CARGO_TOKEN`, `PYPI_TOKEN`, and `VSCE_PAT` remain supported.
The CI release path uses trusted publishing or OIDC where the registry supports it.

## Failure Recovery

If `just release <kind>` has already created a release commit and tag locally, do not run it again.
Inspect the state first.
If the tag is correct, use `just push-release`.

If CI packaging or publish fails after the tag is pushed, fix the underlying problem and rerun the failed workflow or publish step.
Do not create a second tag for the same intended version.

If a registry publish partially succeeds, treat the pushed version as burned unless the registry explicitly supports full rollback for that package.
