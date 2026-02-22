# Types

Type system coverage focuses on TypeScript compatible behavior plus Destack specific extensions.
This includes assignability, generics, type operators, widening commitments, and nominal features like structs and newtypes.

## Inference and mapping matrix

This matrix indexes high-signal Analyze-facing behavior to fixture families.

| Behavior | Primary fixtures |
| --- | --- |
| Literal freshness and widening commitments | `widening/literals.md`, `flow/if/ternary.md`, `operators/satisfies.md` |
| Dynamic and fixed array assignability plus literal commitment | `arrays/dynamic.md`, `arrays/sized.md`, `arrays/literals.md` |
| Cross-module array type forwarding | `arrays/modules.md` |
| Enum backing behavior and cross-module nominal identity | `enums/enum.md`, `enums/modules.md` |
| Newtype constructor and assignability boundaries across modules | `newtypes/newtype.md`, `newtypes/assignability.md`, `newtypes/modules.md` |
| Generic inference and conditional infer semantics | `parameterization/infer.md` |
| TS++ static type and comptime arguments | `parameterization/type-aliases.md`, `parameterization/classes.md`, `parameterization/interfaces.md` |
| Template-literal matching and inference | `template-literals/inference-basic.md`, `template-literals/inference-arguments.md`, `template-literals/inference-numeric.md` |
| Mapped types, key remapping, and modifier propagation | `parameterization/mapped.md`, `parameterization/modifiers.md` |
| Indexed access, `keyof`, and `in` operator behavior | `operators/keyof-in.md`, `objects/index-signatures.md` |
| Cross-module parity for inference and projection paths | `parameterization/infer.md`, `template-literals/modules.md`, `declarations/associated-types/modules.md`, `declarations/associated-comptime/modules.md` |

## Deferred locks

Known implementation gaps are codified as spec fixtures and tracked in `known-failures.txt`.
Current known parity gaps include:
- unannotated exported ternary inference across modules
- mapped associated-comptime projection equivalence in cross-module paths
- `${T}` template-literal argument inference from widened `let string` inputs
- `${T}` template-literal argument inference from non-contextual const ternary unions
