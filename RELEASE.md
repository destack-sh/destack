# Release

Destack releases are tag driven.
The canonical release tag format is `vX.Y.Z`.
Release CI reuses the same release-blocking verification coverage as the scheduled nightly lane and then adds artifact packaging, signatures, installer checks, updater checks, and publishing.
Nightly runs that same release-blocking verification coverage, adds nightly-only canary lanes like fuzz and bench, signs the canary artifacts with the current release key, and publishes a rolling GitHub prerelease from `main`.

## Model

Use the repository root `just` recipes as the public operator surface.
Do not bypass them with ad hoc script invocations unless you are repairing a failed release.
Destack uses one canonical monorepo release version from [VERSION.txt](/Users/florian/symbol/destack/VERSION.txt).
Public project maturity is tracked separately through the single `Status` label in the area README inventories.

| Command | Purpose |
|---------|---------|
| `just bump`, `just bump minor`, `just bump major` | Update `VERSION.txt` and all tracked version files |
| `just release` | Prepare the default patch release commit and tag locally |
| `just validate-release` | Validate tag and tracked versions |
| `just release minor`, `just release major` | Prepare a non-patch release commit and tag locally |
| `just release-push` | Push the current release commit and local release tag |
| `just publish --dry-run` | Dry run the full multi-registry publish fanout |
| `just publish-release` | Publish live using the normal staged release path |
| `just publish-release-local` | Publish live using local CLI binary staging |

## Version Policy

Use stable semver releases deliberately.
Nightly is the high-frequency canary channel for `main`.
Stable releases should be less frequent and intentional.

### Patch

Use `patch` for the normal stable release path.
This is the default for `just release` and `just bump`.
Ship patch releases for fixes, polish, infrastructure changes, compatibility work, and smaller user-visible additions.

### Minor

Use `minor` when the release is a clear external milestone.
This includes notable new user-facing features, new public package surfaces, major capability expansion, or a release you want users to treat as a meaningful step forward.
Minor releases should happen much less often than nightly and somewhat less often than patch.

### Major

Use `major` only for explicit epoch boundaries.
While Destack remains in `0.x`, major releases should be rare.

## Standard Flow

This is the normal operator path.

1. Run `just quick`.
2. Run `just full` if you want local deep validation before tagging.
3. Run `just release`, `just release minor`, or `just release major`.
4. Review the resulting commit and tag.
5. Run `just release-push`.

Pushing the `vX.Y.Z` tag triggers [.github/workflows/release.yml](/Users/florian/symbol/destack/.github/workflows/release.yml).

## Local Validation

Run these commands from the repository root when preparing a release manually.

```sh
just quick
just full
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

## Nightly

Nightly is the canary channel for `main`.
It runs the same release-blocking verification coverage as release and packages the same CLI artifacts.
Nightly signs `manifest.json`, `SHA256SUMS`, `install.sh`, and `install.ps1` with the current release signing key.
Nightly then updates the rolling `nightly` GitHub prerelease from `main`.
Nightly workflow artifacts remain available for debugging:

- `nightly-schemas-${sha}`
- `nightly-cli-assets-${sha}`

The nightly prerelease is a canary channel, not a stable semver release.
Its packaged manifest carries `channel = nightly` and uses the rolling `nightly` release tag.
Nightly should be the fast path for “latest verified main”.
Stable should be the intentional promotion path for a verified commit you want users to adopt.

## CI Release Flow

The release workflow is tag driven and intentionally not `workflow_dispatch` driven.
It validates release metadata first, then runs scoped `full` gates, runtime reusable workflows, artifact packaging, signature generation, installer verification, and publishing.

Important workflow pieces:

- [release.yml](/Users/florian/symbol/destack/.github/workflows/release.yml)

## Release Notes

GitHub releases are the release history for the monorepo.
The release workflow uses GitHub generated release notes for the tag.
Edit the GitHub release manually only when the generated notes miss a user-visible change, migration note, or operator warning.

Version bumps and maturity labels are intentionally separate.
The monorepo version tracks coordinated releases.
Project `Status` tracks local maturity such as `Experimental`, `Alpha`, or `Beta`.
Do not use package local changelogs as a second monorepo release source of truth.
Keep package local changelogs only where a registry requires one.

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
If the tag is correct, use `just release-push`.

If CI packaging or publish fails after the tag is pushed, fix the underlying problem and rerun the failed workflow or publish step.
Do not create a second tag for the same intended version.

If a registry publish partially succeeds, treat the pushed version as burned unless the registry explicitly supports full rollback for that package.
