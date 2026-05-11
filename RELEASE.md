# Release

Destack releases are tag driven.
The canonical release tag format is `vX.Y.Z`.
The monorepo version in [VERSION.txt](VERSION.txt) is the only coordinated release version.

## Commands

Use the repository root `just` recipes as the release operator surface.

| Command | Purpose |
|---------|---------|
| `just bump`, `just bump minor`, `just bump major` | Update `VERSION.txt` and tracked package versions |
| `just release`, `just release minor`, `just release major` | Create the release commit and tag locally |
| `just validate-release` | Validate release metadata and tracked versions |
| `just release-push` | Push the release commit and tag |
| `just publish --dry-run` | Dry run package publishing |
| `just publish-release` | Publish live from staged release artifacts |
| `just publish-release-local` | Publish live with locally staged CLI binaries |

## Standard Flow

This is the normal release path.

1. Run `just quick`.
2. Run `just full` if you want deep local validation before tagging.
3. Run `just release`, `just release minor`, or `just release major`.
4. Review the release commit and tag.
5. Run `just release-push`.

Pushing the `vX.Y.Z` tag triggers [.github/workflows/release.yml](.github/workflows/release.yml).

## Validation

Run these from the repository root when preparing or checking a release.

```sh
just quick
just full
just validate-release
just publish --dry-run
```

Use `just publish --dry-run` for a broad packaging and registry preflight without performing a live publish.

## Nightly

Nightly is the rolling canary release for `main`.
It runs release-blocking verification, packages CLI artifacts, signs release metadata and installers, and updates the rolling `nightly` GitHub prerelease.
Nightly artifacts are for debugging and validation, not stable adoption.

## Release Notes

The release workflow uses GitHub generated release notes for the tag.
Edit the GitHub release manually only when generated notes miss a user-visible change, migration note, or operator warning.

Do not use package local changelogs as a second monorepo release source of truth.
Keep package local changelogs only where a registry requires one.

## Integrity

Release artifacts include `manifest.json`, `SHA256SUMS`, `install.sh`, and `install.ps1`.
CI produces detached armored signatures with the dedicated `RELEASE_GPG_*` key.
The public verification key is [release-signing-public.asc](app/cli/install/release-signing-public.asc).

## Credentials

Publishing commands load credentials from `.env.local` via `just` when run locally.
GitHub Actions uses the `release` environment for the CI path.

Required release credentials are:

- `CARGO_TOKEN`
- `RELEASE_GPG_PRIVATE_KEY`
- `RELEASE_GPG_PASSPHRASE`
- `RELEASE_GPG_KEY_ID`
- `VSCE_PAT`
- `RELEASE_PUBLISH_ZED`
- `ZED_GITHUB_TOKEN`
- `ZED_REGISTRY_PUSH_TO`

For local live publishing outside CI, token-based variables such as `NPM_TOKEN`, `CARGO_TOKEN`, `PYPI_TOKEN`, and `VSCE_PAT` also work.
The CI release path uses trusted publishing or OIDC where the registry supports it.

## Recovery

If `just release <kind>` has already created a release commit and tag locally, inspect the state before doing anything else.
If the tag is correct, use `just release-push`.

If CI packaging or publish fails after the tag is pushed, fix the underlying problem and rerun the failed workflow or publish step.
Do not create a second tag for the same intended version.

If a registry publish partially succeeds, treat the pushed version as burned unless the registry explicitly supports full rollback for that package.
