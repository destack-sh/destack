# generate

Target generation backends for Destack.
This is where the compiler actually finally produces something useful (outside of diagnostics).

## Tale of Two Backends

Destack has two main generation paths (DIR and MIR), and they are pretty different:

| Backend | Input | Output | Use Case |
|---------|-------|--------|----------|
| `js/` | DIR | JavaScript and TypeScript target outputs | Web and generic JS hosts |
| `native/` | MIR | Native object files, WASM | Performance-critical, embedded, WASM |

The JS backend works directly from DIR because its internal pipeline is semantic planning, emission, assembly, and printing.
The native backend needs MIR because it eventually generates machine code.

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
│ native/   │
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
