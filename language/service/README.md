# Service

`destack_service` owns workspace rooted language tooling orchestration.
It applies file updates, drives incremental analysis, refreshes configuration state, and executes workspace queries.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_service
just language/test-query
just language/test-lsp

# clean gate
just language/quick

# exhaustive gate
just language/full
```
