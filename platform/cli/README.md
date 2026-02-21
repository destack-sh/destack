# cli

Destack command-line interface (`destack`, `ds`, `dsc`, `dsx`).
The `destack` binary for working with Destack projects, with short aliases:
- `ds` for general commands
- `dsc` for `destack build`
- `dsx` for `destack run`

## npm distribution

The CLI is published on npm as `@destack/cli`.
The main package dispatches to platform-specific optional dependency packages that contain prebuilt Rust binaries.
The npm shims expose `destack`, `ds`, `dsc`, and `dsx`.
The direct curl and PowerShell installers are in `install/install.sh` and `install/install.ps1`.

Supported npm binary packages:

- `@destack/cli-darwin-arm64`
- `@destack/cli-darwin-x64`
- `@destack/cli-linux-arm64-gnu`
- `@destack/cli-linux-x64-gnu`
- `@destack/cli-win32-x64-msvc`

Stage npm binaries from release artifacts before publish:

```sh
just platform/stage-cli-binaries-from-artifacts
```

This command validates checksums before staging binaries into npm package directories.

Publish platform packages first, then publish `@destack/cli`.
You can run the full npm publish flow from the repository root with `just platform/publish-cli`.
You can run the integrated developer release flow with `destack dev release`.
You can stage a Zed registry PR from the same command with `--publish-zed`.

Recommended preflight command from repository root:

```sh
just platform/preflight-cli-publish
```

If you are preparing artifacts locally, run this build and packaging flow first:

```sh
just platform/build-cli-binaries
just platform/package-cli-release
just platform/stage-cli-binaries-from-artifacts
```

Set `DESTACK_RELEASE_TARGETS` to space-separated Rust target triples when you want to stage a local subset.

Integrated release examples:

```sh
# preflight + dry-run publish
destack dev release

# bump patch version, then preflight + dry-run publish
destack dev release --bump patch

# live publish after preflight
destack dev release --publish-live

# dry-run npm, vscode, and zed registry publish
destack dev release --publish-zed

# live npm, vscode, and zed registry publish
DESTACK_ZED_REGISTRY_PUSH_TO=your-github-user/extensions destack dev release --publish-live --publish-zed
```

Live Zed publish requires authenticated `gh` access and a push target fork in `DESTACK_ZED_REGISTRY_PUSH_TO`.
Local `just` commands load repository `.env.local`, so you can keep publish tokens there.

Publish order:

```sh
cd platform/cli/npm-darwin-arm64 && npm run publish:live
cd platform/cli/npm-darwin-x64 && npm run publish:live
cd platform/cli/npm-linux-arm64-gnu && npm run publish:live
cd platform/cli/npm-linux-x64-gnu && npm run publish:live
cd platform/cli/npm-win32-x64-msvc && npm run publish:live
cd platform/cli && npm run publish:live
```

## Layout

| Path | Purpose | Description |
| --- | --- | --- |
| `main.rs` | Entry | Primary CLI entrypoint for the `destack` binary. |
| `entry.rs` | Entry | Alias handling (`ds`, `dsc`, `dsx`) and dispatch. |
| `cli.rs` | Arguments | CLI argument parsing. |
| `command/` | Commands | Subcommands and their argument structs. |
| `common/` | Shared | Shared CLI utilities (diagnostics, formatting, reports). |
| `pipeline/` | Pipeline | Shared orchestration helpers (targets, runtime, workspace). |
| `console/` | Output | Terminal output utilities. |

## Commands

| Command | Purpose |
| --- | --- |
| `check` | Type-check sources (alias: `typecheck`). |
| `lint` | Lint sources (alias of check with linting). |
| `format` | Format source files (alias: `fmt`). |
| `build` | Compile sources for a target (alias: `compile`). |
| `run` | Compile and run a module or script (alias: `exec`). |
| `eval` | Evaluate inline code. |
| `repl` | Start a REPL session (stub). |
| `clean` | Remove build outputs and caches. |
| `cache` | Show cache locations and settings. |
| `info` | Show workspace and target info. |
| `config` | Show resolved config and targets. |
| `targets` | List configured build targets. |
| `doctor` | Show environment and workspace diagnostics (alias: `env`). |
| `explain` | Explain a diagnostic or lint rule (or list diagnostics). |
| `completions` | Generate shell completions. |
| `version` | Show version info. |
| `task` | Run tasks from `dsconfig.json`. |
| `test` | Run tests (stub). |
| `bench` | Run benchmarks (stub). |
| `doc` | Generate docs (stub). |
| `lsp` | Run the language server. |
| `daemon` | Manage the background daemon service. |
| `dev` | Developer workflows, including integrated release flows. |

`run` resolves `dsconfig.json` tasks first, then `package.json` scripts when the argument is not a file path.

## Common flags

| Flag | Purpose |
| --- | --- |
| `--output-format <text|json>` | Emit structured JSON output for tooling. |
| `--json` | Shorthand for `--output-format json`. |
| `--cwd <dir>` | Set the working directory. |
| `--config <path>` | Use a specific dsconfig.json. |
| `--workspace <dir>` | Set the workspace root. |
| `--cache-dir <dir>` | Override the cache directory. |
| `--workers <n>` | Number of worker threads. |
| `--watch` | Watch mode for supported commands. |
| `--dev` | Dev mode (not yet implemented). |
| `--timings` | Emit timing diagnostics when supported. |
| `--timings-top <n>` | Limit timing tag output to the top N entries. |
| `--timings-min-ms <ms>` | Filter timing tags shorter than the threshold. |
| `--profile` | Emit profiling diagnostics when supported. |
| `--color <auto|always|never>` | Override ANSI color output. |
| `--log <error|warn|info|debug|trace>` | Enable tracing logs. |

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

Generate the CLI report schema with one of these commands.
Run these commands from the repository root.

```sh
just platform/generate-schema

# or
cargo run -p destack_cli --features schema --bin generate-cli-schema --release > platform/cli/generated/cli-report.schema.json
```
