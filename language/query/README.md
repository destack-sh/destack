# Query

Semantic tooling queries and presentation formatting for the Destack language toolchain.

This crate owns IDE-facing semantic reads over `destack_workspace`, including navigation, assists, refactors, hover, signature help, and related presentation formatting.

## Scope

`destack_query` depends on `destack_workspace` for semantic state.
It does not own programs, sessions, artifacts, or frontier state itself.

This crate exists to keep tooling reads and formatting out of the workspace state layer.

## Testing

Run these commands from the repository root.

```sh
cargo check -p destack_query
cargo test -p destack_test --test query
```
