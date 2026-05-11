# cli installer

The installer scripts for Destack CLI live in this directory.
They are designed to be served by `destack.sh` and sourced from this repository release pipeline.

## scripts

The shell installer targets macOS and Linux.

- `install.sh`
- `package-release-artifacts.sh`
- `validate-release-artifacts.sh`
- `test-install.sh`
- `stage-npm-binaries-from-artifacts.sh`

The PowerShell installer targets Windows.

- `install.ps1`

## environment variables

These variables can override installer behavior.

| Variable | Description | Default |
| --- | --- | --- |
| `DESTACK_VERSION` | Release version or `latest`. | `latest` |
| `DESTACK_INSTALL` | Destination binary directory. | `~/.destack/bin` |
| `DESTACK_NO_MODIFY_PATH` | Skip PATH updates when set to `1`. | `0` |
| `DESTACK_REPOSITORY` | Release repository slug. | `destack-sh/destack` |
| `DESTACK_RELEASE_BASE_URL` | Release download base URL. | `https://github.com/<repo>/releases/download` |
| `DESTACK_API_BASE_URL` | API base URL for latest release lookup. | `https://api.github.com/repos/<repo>` |
| `DESTACK_GITHUB_TOKEN` | Optional GitHub token for API and asset downloads. | unset |
| `DESTACK_CURL_USER_AGENT` | Optional request user agent override. | `destack-cli-installer` |

## expected release assets

The install scripts expect release artifacts with these names.

### unix archives

The shell installer expects tar archives.

- `destack-<version>-aarch64-apple-darwin.tar.gz`
- `destack-<version>-x86_64-apple-darwin.tar.gz`
- `destack-<version>-aarch64-unknown-linux-gnu.tar.gz`
- `destack-<version>-x86_64-unknown-linux-gnu.tar.gz`

### windows archive

The PowerShell installer expects a zip archive.

- `destack-<version>-x86_64-pc-windows-msvc.zip`

### checksum file

Both installers verify checksums from a shared file.

- `SHA256SUMS`

### update manifest

Release artifacts also include a machine-readable update manifest.

- `manifest.json`

The manifest maps target triples to archive names and sha256 hashes.
It also records the release tag and channel that the updater should bind to.
Future in-cli auto-update uses this file as the canonical release metadata source.

Release and nightly CI both publish detached signatures for all metadata and installer scripts.

- `manifest.json.asc`
- `SHA256SUMS.asc`
- `install.sh.asc`
- `install.ps1.asc`
- `release-signing-public.asc`

The cli `destack update` standalone lane verifies `manifest.json.asc`, `SHA256SUMS.asc`, and the installer script signature against this key during update apply.
Use `destack update --check --verify` to run metadata signature verification without applying the update.

Stable releases use a `vX.Y.Z` release tag and `channel = stable`.
Nightly uses the rolling `nightly` prerelease tag and `channel = nightly`.

## release wiring

The scripts are source controlled here and should be deployed by release automation.
A production `destack.sh/install` endpoint should serve or redirect to `install.sh` from this repository.

## ci publish prerequisites

The `release.yml` publish job requires trusted publishing to stay enabled for npm and PyPI.
The job stages npm binaries from `release-cli-assets` and then publishes npm packages through `just publish ""`.
If `RELEASE_PUBLISH_ZED` is enabled for tag releases, the job also requires `ZED_GITHUB_TOKEN` and `ZED_REGISTRY_PUSH_TO`.
The create-release lane signs `manifest.json` and `SHA256SUMS` with the dedicated `RELEASE_GPG_*` key.

## local secret loading

Local `just` commands load `.env.local` from the repository root.
Use `.env.local.example` as the template for local token setup.
The file is gitignored and should not be committed.

## local release checks

You can package and validate local release artifacts with these commands from the repository root.

```sh
bash app/cli/install/package-release-artifacts.sh
bash app/cli/install/validate-release-artifacts.sh
bash app/cli/install/test-install.sh
bash app/cli/install/stage-npm-binaries-from-artifacts.sh
just app/preflight-cli-publish
```

Set `DESTACK_RELEASE_TARGETS` to a space-separated target list when running local subset checks.
