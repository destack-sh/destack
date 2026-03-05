# VSCode LSP Tests

This folder contains deterministic VSCode LSP smoke tests for the Destack extension.
The suite validates extension activation, command registration, and language feature wiring.
The `tests-lsp` name reflects that these tests verify LSP behavior through VSCode integration.
This keeps VSCode LSP tests separate from grammar fixtures and Rust-side integration tests.

By default the runner writes `.vscode/settings.json` to launch `tools/mock-lsp.js`.
That mock server is intentionally local and dependency free so LSP tests run without a built Destack binary.

Set `DESTACK_VSCODE_REAL_SERVER_COMMAND` to run the real server integration matrix.
When this command points to the local workspace target binary (`target/debug/destack` or `target/release/destack`), the runner auto-builds `destack_cli` first for hermetic runs.
Set `DESTACK_VSCODE_REAL_SERVER_AUTO_BUILD=0` to disable this behavior.
The real matrix covers diagnostics, multi-file edits, definitions across edits, rename participation, and workspace diagnostic cancellation.

The suite is split into focused capability files.
Bootstrap tests live in `suite/bootstrap.test.ts`.
Real server tests live in `suite/diagnostics.test.ts`, `suite/commands.test.ts`, `suite/edits.test.ts`, `suite/rename.test.ts`, and `suite/workspace.test.ts`.
Shared test helpers live in `suite/tests.ts`.
Shared applied-LSP fixture payloads are loaded from `service/lsp/fixtures` through the test runner environment.

```sh
DESTACK_VSCODE_REAL_SERVER_COMMAND=/absolute/path/to/destack \
DESTACK_VSCODE_REAL_SERVER_ARGS='["lsp"]' \
bun run test:lsp
```

The smoke suite remains focused on VSCode integration behavior and process wiring.
