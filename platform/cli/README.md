# cli

Destack command-line interface.
The `destack` binary for compiling, formatting, and running Destack projects.

## Layout

| Path | Purpose | Description |
| --- | --- | --- |
| `main.rs` | Entry | Entry point for the CLI. |
| `cli.rs` | Arguments | CLI argument parsing. |
| `command/` | Commands | Subcommands like compile, parse, lex, resolve, etc. |
| `console/` | Output | Terminal output utilities. |
