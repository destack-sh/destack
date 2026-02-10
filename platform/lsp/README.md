# lsp

Destack Language Server Protocol implementation.
Provides classic IDE features: diagnostics, completions, go-to-definition, semantic highlighting.

## Integration

The LSP server owns its workspace session in process.
It uses `destack_workspace` and `destack_compiler` directly through an LSP local workspace driver.
Editor edits are applied to an overlay file system and then analyzed in the same process.
The LSP crate does not depend on `destack_daemon`.

## Relationship To Daemon

The daemon is a separate service used by CLI and automation flows.
Both daemon and LSP are clients of the same `destack_workspace` query layer.
This keeps editor behavior consistent with daemon backed workflows without coupling LSP transport to daemon IPC.

## Change Flow

The LSP change flow is:

- apply overlay edits to the session file system
- bump file and module versions
- enqueue incremental compile tasks
- publish diagnostics scoped to the edited files
