# Formatter Conformance Tests

Formatter conformance tests measure how close formatter behavior is to the external formatter we intentionally track.

## Scope

The formatter conformance oracle is `oxfmt`.
This lane exists to keep the shared JS and TS surface structurally aligned with the tracked external formatter.
It is not a broader formatter ecosystem suite.

## Running

Run the formatter conformance suite with:

```sh
cargo test --release --test conformance-formatter -- --oxfmt
```
