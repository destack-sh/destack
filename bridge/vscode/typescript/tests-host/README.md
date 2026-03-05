# VSCode Host Tests

This folder contains deterministic extension host smoke tests for the Destack VSCode extension.
The suite validates extension activation, command registration, and language feature wiring.
The `tests-host` name follows VSCode terminology for extension host integration tests.
This keeps host-process tests separate from grammar fixtures and Rust-side integration tests.

By default the runner writes `.vscode/settings.json` to launch `tools/mock-lsp.js`.
That mock server is intentionally local and dependency free so host tests run without a built Destack binary.

Set `DESTACK_VSCODE_REAL_SERVER_COMMAND` to run the real server integration matrix.
When this command points to the local workspace target binary (`target/debug/destack` or `target/release/destack`), the runner auto-builds `destack_cli` first for hermetic runs.
Set `DESTACK_VSCODE_REAL_SERVER_AUTO_BUILD=0` to disable this behavior.
The real matrix covers diagnostics, multi-file edits, definitions across edits, rename participation, and workspace diagnostic cancellation.

The suite is split into focused files.
Smoke tests live in `suite/smoke.test.ts`.
Real server tests live in `suite/diag.test.ts`, `suite/command.test.ts`, `suite/flow.test.ts`, `suite/rename.test.ts`, and `suite/workspace.test.ts`.
Shared test helpers live in `suite/support.ts`.

```sh
DESTACK_VSCODE_REAL_SERVER_COMMAND=/absolute/path/to/destack \
DESTACK_VSCODE_REAL_SERVER_ARGS='["lsp"]' \
bun run test:host
```

The smoke suite remains focused on extension host behavior and process wiring.
