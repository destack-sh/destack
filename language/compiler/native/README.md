# native

Native and WebAssembly code generation using [Cranelift](https://cranelift.dev/).
Takes MIR and produces object files (`.o`) or WASM modules (`.wasm`).

## Why Cranelift?

Cranelift is a fast, portable code generator originally built for Firefox's WASM engine.
Not as optimizing as LLVM, but compiles much faster and produces good code for most use cases.

For Destack, compilation speed matters more than the last 5% of runtime performance.
We do our own optimizations in MIR anyway (see [compiler/README.md](../README.md)).
We can always add an LLVM backend later for release builds if needed.

## Pipeline

```text
MIR → Cranelift IR → Machine Code → Object File
      (lower)        (compile)      (emit)
```

1. **Lower** (`lower/`): Translate MIR functions to Cranelift IR
2. **Compile**: Cranelift compiles to machine code
3. **Emit**: Output as object file or WASM module

## MIR to Cranelift

MIR maps fairly directly to Cranelift IR since both are SSA-based:

| MIR | Cranelift |
|-----|-----------|
| Functions | `Function` with signature |
| Blocks | `Block` (basic blocks) |
| SSA values | `Value` |
| `call` | `call` / `call_indirect` |
| `return` | `return` |
| `branch` | `brif` |
| `switch` | `br_table` |
| `load` / `store` | `load` / `store` |
| Intrinsics | Native instructions or libcalls |

The lowerer handles type mapping (MIR types to Cranelift types) and calling convention translation.

## Debug Symbols

Native debug info is emitted via DWARF using Cranelift's debug support.
Lower provides source spans, function names, local variable names, scopes, and
type descriptors; Cranelift maps these to DWARF line tables and debug info
entries (DIEs). This enables source-level debugging in lldb/gdb.

## Optimization Levels

Cranelift supports three optimization levels:

| Level | Setting | Description |
|-------|---------|-------------|
| O0 | `none` | No optimization, fastest compile |
| O1/O2 | `speed` | Standard optimizations |
| O3/O4 | `speed_and_size` | Optimize for both speed and size |

Debug builds use `none` for fast iteration.
Release builds use `speed` or `speed_and_size`.
O4 uses the same backend setting as O3 and relies on additional MIR and LTO work for extra gains.
(The difference between O1 and O2 is mostly in our own MIR optimizations.)
