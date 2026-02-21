# unicode

Unicode property tables and utilities.
Generated tables for identifier validation (XID_Start, XID_Continue) and display width.

## Testing

Run these from the repository root.

### Quick local loop

```sh
cargo test -p destack_unicode
```

### Parser integration coverage

```sh
cargo test -p destack_parser
cargo test -p destack_test --test smoke -- --parser
```
