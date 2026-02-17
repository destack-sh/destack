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

## release wiring

The scripts are source controlled here and should be deployed by release automation.
A production `destack.sh/install` endpoint should serve or redirect to `install.sh` from this repository.

## ci publish prerequisites

The `release.yml` publish job requires `NPM_TOKEN` and `VSCE_PAT` secrets in the `release` environment.
The job stages npm binaries from `release-cli-assets` and then publishes npm packages through `just publish ""`.
If `publish_zed` is enabled for workflow dispatch, the job also requires `ZED_GITHUB_TOKEN` and `ZED_REGISTRY_PUSH_TO`.

## local secret loading

Local `just` commands load `.env.local` from the repository root.
Use `.env.local.example` as the template for local token setup.
The file is gitignored and should not be committed.

## local release checks

You can package and validate local release artifacts with these commands from the repository root.

```sh
bash platform/cli/install/package-release-artifacts.sh
bash platform/cli/install/validate-release-artifacts.sh
bash platform/cli/install/test-install.sh
bash platform/cli/install/stage-npm-binaries-from-artifacts.sh
just platform/preflight-cli-publish
```

Set `DESTACK_RELEASE_TARGETS` to a space-separated target list when running local subset checks.
