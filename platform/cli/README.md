# cli

Destack command-line interface (`destack`, `ds`, `dsc`, `dsx`).
The `destack` binary for working with Destack projects, with short aliases:
- `ds` for general commands
- `dsc` for `destack build`
- `dsx` for `destack run`

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

Generate the CLI report schema with:
Run these commands from the repository root.

```sh
just platform/generate-schema

# or
cargo run -p destack_cli --features schema --bin generate-cli-schema --release > platform/cli/generated/cli-report.schema.json
```
