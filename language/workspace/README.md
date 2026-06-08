# Workspace

`destack_workspace` owns workspace rooted language tooling orchestration.
It applies file updates, drives incremental analysis, refreshes configuration state, and executes workspace queries.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_workspace
just language/test-query

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
