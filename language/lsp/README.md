# lsp

Destack Language Server Protocol implementation.
This crate owns protocol mapping, request lifecycle handling, progress, cancellation, and diagnostics sequencing for the Destack language server.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_lsp
just language/test-lsp

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
