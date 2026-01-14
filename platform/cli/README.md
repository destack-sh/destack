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
| `run` | Compile and run a module (alias: `exec`). |
| `clean` | Remove build outputs and caches. |
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

## Common flags

| Flag | Purpose |
| --- | --- |
| `--output-format <text|json>` | Emit structured JSON output for tooling. |
| `--json` | Shorthand for `--output-format json`. |
| `--cwd <dir>` | Set the working directory. |
| `--config <path>` | Use a specific dsconfig.json. |
| `--workspace <dir>` | Set the workspace root. |
| `--workers <n>` | Number of worker threads. |
| `--watch` | Watch mode (not yet implemented). |
| `--dev` | Dev mode (not yet implemented). |
| `--timings` | Emit timing diagnostics when supported. |
| `--profile` | Emit profiling diagnostics when supported. |
| `--color <auto|always|never>` | Override ANSI color output. |
| `--log <error|warn|info|debug|trace>` | Enable tracing logs. |
