# cli

Destack command-line interface (`destack`, `ds`, `dsc`, `dsx`).
The `destack` binary for working with Destack projects, with short aliases:

- `ds` for general commands
- `dsc` for `destack build`
- `dsx` for `destack run`

## npm distribution

The CLI is published on npm as `@destack/cli`.
The main package dispatches to target-specific optional dependency packages that contain prebuilt Rust binaries.
The npm shims expose `destack`, `ds`, `dsc`, and `dsx`.
The direct curl and PowerShell installers are in `install/install.sh` and `install/install.ps1`.

Supported npm binary packages:

- `@destack/cli-darwin-arm64`
- `@destack/cli-darwin-x64`
- `@destack/cli-linux-arm64-gnu`
- `@destack/cli-linux-x64-gnu`
- `@destack/cli-win32-x64-msvc`

## Release

Stage npm binaries from release artifacts before publish:

```sh
just app/stage-cli-binaries-from-artifacts
```

This command validates checksums before staging binaries into npm package directories.

Publish target packages first, then publish `@destack/cli`.
You can run the integrated developer release flow with `destack dev release`.
You can stage a Zed registry PR from the same command with `--publish-zed`.

Recommended preflight command from repository root:

```sh
just app/preflight-cli-publish
```

If you are preparing artifacts locally, run this build and packaging flow first:

```sh
just app/build-cli-binaries
just app/package-cli-release
just app/stage-cli-binaries-from-artifacts
```

Set `DESTACK_RELEASE_TARGETS` to space-separated Rust target triples when you want to stage a local subset.

Integrated release examples:

```sh
# preflight + dry-run publish
destack dev release

# bump patch version, then preflight + dry-run publish
destack dev release --bump patch

# live publish after preflight
destack dev release --publish

# dry-run npm, vscode, and zed registry publish
destack dev release --publish-zed

# live npm, vscode, and zed registry publish
DESTACK_ZED_REGISTRY_PUSH_TO=your-github-user/extensions destack dev release --publish --publish-zed
```

Live Zed publish requires authenticated `gh` access and a push target fork in `DESTACK_ZED_REGISTRY_PUSH_TO`.
Local `just` commands load repository `.env.local`, so you can keep publish tokens there.

## JSON output

All commands that support JSON output emit a common report envelope:

```json
{
  "schema_version": 3,
  "command": "config",
  "status": "success",
  "exit_code": 0,
  "summary": null,
  "diagnostics": null,
  "error": null,
  "stats": null,
  "data": {}
}
```

List payloads use:

```json
{ "items": [], "total": 0 }
```

Grouped list payloads use:

```json
{ "groups": [], "total_groups": 0, "total_items": 0 }
```

Generate the CLI report schema from the repository root.

```sh
just app/generate-schema
```

## Testing

Run these from the repository root.

```sh
cargo test -p destack_cli
just app/check-quick
just app/check-full
```
