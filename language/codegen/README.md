# codegen

Code generation backends for Destack.
This is where the compiler actually finally produces something useful (outside of diagnostics).

## Tale of Two Backends

Destack has two main codegen paths (DIR and MIR), and they are pretty different:

| Backend | Input | Output | Use Case |
|---------|-------|--------|----------|
| `js/` | DIR | JavaScript/TypeScript source | Web, Node, Bun, Deno |
| `cranelift/` | MIR | Native object files, WASM | Performance-critical, embedded, WASM |

The JS backend works directly from DIR (the semantic IR) because we're just "printing code".
The Cranelift backend needs MIR (the machine IR) because it's generating actual machine instructions (ish).

```text
         ┌─────────────┐
         │     DIR     │
         └──────┬──────┘
                │
    ┌───────────┴───────────┐
    │                       │
    ▼                       ▼
┌───────┐               ┌───────┐
│  js/  │               │ Lower │
└───┬───┘               └───┬───┘
    │                       │
    ▼                       ▼
┌───────────┐           ┌───────┐
│  .js/.ts  │           │  MIR  │
└───────────┘           └───┬───┘
                            │
                            ▼
                      ┌───────────┐
                      │ cranelift │
                      └─────┬─────┘
                            │
                ┌───────────┴───────────┐
                ▼                       ▼
          ┌──────────┐            ┌──────────┐
          │  .wasm   │            │   .o     │
          └──────────┘            └──────────┘
```

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_codegen_js
cargo test -p destack_codegen_lib
cargo test -p destack_codegen_native
just language/test-emit

# clean gate
just language/quick

# exhaustive gate
just language/full
```
