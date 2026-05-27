# Release

Destack releases are (Git) tag driven:
 - The canonical release tag format is `vX.Y.Z`.
 - The monorepo version for all components is in [VERSION.txt](VERSION.txt).

## Channels

Destack uses stack-wide release channels.

| Channel | Meaning |
|---------|---------|
| `stable` | Public supported releases from `vX.Y.Z` tags |
| `nightly` | Signed early-access builds from `main` published on the rolling `nightly` prerelease tag |
| `canary` | Internal validation builds, not a public installer channel |

## Commands

Use the repository root's `just` recipes.

| Command | Purpose |
|---------|---------|
| `just bump`, `just bump minor`, `just bump major` | Update `VERSION.txt` and tracked package versions |
| `just release`, `just release minor`, `just release major` | Create the release commit and tag locally |
| `just validate-release` | Validate release metadata and tracked versions |
| `just release-push` | Push the release commit and tag |
| `just publish --dry-run` | Dry run package publishing |
| `just publish-release` | Publish live from staged release artifacts |
| `just publish-release-local` | Publish live with locally staged CLI binaries |

Run the standard flow.

1. Run `just check-quick`.
2. Run `just check-full`.
3. Run `just release`, `just release minor`, or `just release major`.
4. Review the release commit and tag.
5. Run `just release-push`.

Pushing the `vX.Y.Z` tag triggers [.github/workflows/release.yml](.github/workflows/release.yml).

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
- `CARGO_TOKEN`
- `RELEASE_GPG_PRIVATE_KEY`
- `RELEASE_GPG_PASSPHRASE`
- `RELEASE_GPG_KEY_ID`
- `VSCE_PAT`
- `RELEASE_PUBLISH_ZED`
- `ZED_GITHUB_TOKEN`
- `ZED_REGISTRY_PUSH_TO`

For local live publishing outside CI, token-based variables such as `NPM_TOKEN`, `CARGO_TOKEN`, `PYPI_TOKEN`, and `VSCE_PAT` also work.
(The CI release path uses trusted publishing or OIDC where the registry supports it.)
