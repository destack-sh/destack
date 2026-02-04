# lsp

Destack Language Server Protocol implementation.
Provides classic IDE features: diagnostics, completions, go-to-definition, semantic highlighting.

## Integration

The LSP server is a thin client over the daemon session.
The daemon is auto spawned per workspace and accessed over local IPC.
Editor edits are applied to the overlay file system and forwarded as file change events.
The daemon performs incremental compilation and returns diagnostics and query data.

## Change Flow

The LSP change flow is:

- apply overlay edits to the session file system
- bump file and module versions
- enqueue incremental compile tasks
- publish diagnostics scoped to the edited files