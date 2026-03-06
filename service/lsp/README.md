# lsp

Destack Language Server Protocol implementation.
This crate owns protocol mapping, request lifecycle handling, progress, cancellation, and diagnostics sequencing for the Destack language server.

## Testing

Run these commands from the repository root.

### Quick local loop

```sh
cargo test -p destack_lsp
```

### Applied LSP coverage

```sh
just language/test-lsp
```
