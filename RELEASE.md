# Release

Destack releases are (Git) tag driven:
 - The canonical release tag format is `vYEAR.MONTH.MICRO`.
 - The root [destack.json](destack.json) declares the complete Destack distribution and workspace.
 - Its `version` identifies the integrated release train.
 - Its `products.destack.stability` describes the complete Destack distribution.

Destack advances `MICRO` for each weekly release and resets it to zero when the UTC month changes.

## Stability

Each Package or Product declares its own Stability.
The root manifest's `destack` Product applies Stability only to the complete distribution.

| Stability | Meaning |
|-----------|---------|
| `experimental` | Releases with no compatibility guarantee |
| `alpha` | Releases intended for early use |
| `beta` | Releases approaching compatibility guarantees |
| `stable` | Releases governed by compatibility guarantees |

Tagged packages use their resolved Stability as the npm distribution tag, except `stable`, which publishes as `latest`.
Experimental, alpha, and beta GitHub and VS Code releases are prereleases.

## Channels

Channels describe how a build is distributed independently of its Stability.

| Channel | Meaning |
|---------|---------|
| `release` | Versioned build published from a release tag |
| `nightly` | Signed early-access builds from `main` published on the rolling `nightly` prerelease tag |
| `canary` | Internal validation builds, not a public installer channel |

## Commands

Use the repository root's `just` recipes.

| Command | Purpose |
|---------|---------|
| `just next-version` | Advance the workspace and tracked package versions |
| `just set-version YEAR.MONTH.MICRO` | Set an explicit version |
| `just release` | Create the next weekly release commit and tag locally |
| `just validate-release` | Validate release metadata and tracked versions |
| `just release-push` | Push the release commit and tag |
| `just publish --dry-run` | Dry run package publishing |
| `just publish-release` | Build and publish all packages live |
| `just publish-release-local` | Publish live with locally staged CLI binaries |

Run the standard flow.

1. Run `just check-quick`.
2. Run `just check-full`.
3. Run `just release`.
4. Review the release commit and tag.
5. Run `just release-push`.

Pushing the `vYEAR.MONTH.MICRO` tag triggers [.github/workflows/release.yml](.github/workflows/release.yml).

## Validation

Run these from the repository root when preparing or checking a release.

```sh
just check-quick
just check-full
just validate-release
just publish --dry-run
```

Use `just publish --dry-run` for a broad packaging and registry preflight without performing a live publish.

## Credentials

The publishing commands load credentials from `.env.local` via `just` when run locally, and GitHub Actions uses the `release` environment for the CI path.
The required release credentials are:
- `RELEASE_GPG_PRIVATE_KEY`
- `RELEASE_GPG_PASSPHRASE`
- `RELEASE_GPG_KEY_ID`
- `VSCE_PAT`
- `RELEASE_PUBLISH_ZED`
- `ZED_GITHUB_TOKEN`
- `ZED_REGISTRY_PUSH_TO`

For local live publishing outside CI, token-based variables such as `NPM_TOKEN` and `VSCE_PAT` also work.
(The CI release path uses trusted publishing or OIDC where the registry supports it.)
