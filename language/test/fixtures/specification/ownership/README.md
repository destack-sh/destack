# Ownership

Explicit control over references, values, and ownership.

TypeScript doesn't distinguish references from values. Destack adds opt-in
explicit control, enabling a spectrum from TypeScript simplicity to Rust-level control.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `references/` | Reference types (`&T`, `&mut T`) |
| `values/` | Value types (`^T`, `^var T`) |
| `mutability/` | Mutability modifiers (`const`, `var`) |
| `verification/` | Borrow, lifetime, and move verification |