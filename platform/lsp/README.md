# lsp

Destack Language Server Protocol implementation.
Provides classic IDE features: diagnostics, completions, go-to-definition, semantic highlighting.

## Integration

The LSP server owns its workspace session in process.
It uses `destack_service` as a local workspace orchestration layer.
Editor edits are applied to an overlay file system and then analyzed in the same process.
The LSP crate does not depend on `destack_daemon`.

## Change Flow

The LSP change flow is:

- apply overlay edits to the session file system
- bump file and module versions
- enqueue incremental compile tasks
- publish diagnostics scoped to the edited files
