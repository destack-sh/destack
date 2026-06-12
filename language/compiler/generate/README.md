# generate

JavaScript generation for Destack.
Native code generation lives next to this directory in `../native`.

## Backends

Destack has two main generation paths (DIR and MIR), and they are pretty different:

| Backend | Input | Output | Use Case |
|---------|-------|--------|----------|
| `js/` | DIR | JavaScript and TypeScript target outputs | Web and generic JS hosts |
| `../native/` | MIR | Native object files, WASM | Performance-critical, embedded, WASM |

The JS backend works directly from DIR because its internal pipeline is semantic planning, emission, assembly, and printing.
The native backend needs MIR because it eventually generates machine code.

## Testing

Run these from the repository root.

```sh
cargo test -p destack_codegen_js
just language/check-quick
just language/check-full
```
