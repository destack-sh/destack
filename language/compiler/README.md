# Compiler

The main job of the compiler is to turn _sources_ into final _outputs_ for various _targets_.
The compiler (mostly) operates over our three primary IRs - AST, DIR, MIR - and most of the actually interesting stuff happens over DIR (resolution, analysis, elaboration) and MIR (lowering, optimization, codegen).

## Structure

The compiler is organized around artifact-producing phases.

| Path | Artifact |
| --- | --- |
| `compile/` | coordinator, artifact access, diagnostics |
| `bind/` | `DirParsed` / `Data` -> `DirBound` |
| `import/` | `DirBound` -> `DirImported` |
| `expand/` | `DirBound` + `DirImported` -> `DirExpanded` |
| `export/` | `DirExpanded` -> `DirExported` |
| `resolve/` | `DirImported` + `DirExpanded` + `DirExported` -> `DirResolved` |
| `check/` | `DirExpanded` + `DirExported` + `DirResolved` -> `DirChecked` |
| `materialize/` | `DirChecked` + macro state -> `DirMaterialized` |
| `elaborate/` | `DirMaterialized` -> `DirElaborated` |
| `lower/` | `DirElaborated` -> `MirLowered` |
| `verify/` | `MirLowered` -> `MirVerified` |
| `optimize/` | `MirVerified` + `MirLowered` -> `MirOptimized` |
| `generate/` | compiler artifacts -> module outputs |
| `link/` | module outputs -> package outputs |
| `library/` | compiler-owned bundled library access |
| `common/` | shared helpers only when truly shared |

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_compiler
cargo test -p destack_test --test smoke -- --compiler
just language/test-specification
just language/test-query

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
