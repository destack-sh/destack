# Service

`destack_service` owns workspace rooted language tooling orchestration.
It applies file updates, drives incremental analysis, refreshes configuration state, and executes workspace queries.

## Testing

Run these from the repository root.

### Quick local loop

```sh
cargo test -p destack_service
```

### Query integration coverage

```sh
just language/test-query
```
