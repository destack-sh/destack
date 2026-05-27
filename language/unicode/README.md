# unicode

Unicode property tables and utilities.
Generated tables for identifier validation (XID_Start, XID_Continue) and display width.

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_unicode
cargo test -p destack_parser
cargo test -p destack_test --test smoke -- --parser

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
