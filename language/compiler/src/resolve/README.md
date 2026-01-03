# Resolve

Resolve connects modules into a graph and makes every cross-module reference concrete.
Resolve is the first phase that treats the program as a system rather than isolated trees.
Resolve runs per profile and depends only on Bind outputs.

## Objectives

Resolve is strictly **TypeScript compliant** for imports and exports.
It avoids cross-module deadlocks and unstable cycles.
It preserves Destack extensions without changing TS semantics.

Resolve produces three concrete outcomes that the rest of the pipeline depends on:
1. **Symbol targets** for every import, export, and reference.
2. **Export tables** keyed by `(SymbolSpace, StaticKey)` for every module.
3. **A stable module graph** with module declarations and builtin libs integrated.

## Pipeline

Resolve sits between Bind and Analyze and operates per profile.
Profiles change which libs exist and how specifiers resolve, so we have to fork the DIR for each profile.

```
Bind ───► Resolve ───► Analyze ───► Elaborate ───► Execute ───► Lower
  │         │
  │         ├─► module graph, exports, and symbol targets
  │         └─► per-profile DIR
```

## Tasks

Resolve is task-driven, and the phase split avoids cycles while keeping dependencies explicit:

```
ResolveModulePrepare
    │
    ├─► build export table
    │
    ├─► ResolveModuleDirect
    │      │
    │      ├─► resolve dependencies
    │      ├─► build global symbol table
    │      └─► finalize export targets
    │
    └─► ResolveModuleCanonical
```

ResolveBuiltins and ResolveLibs run before any module-level Resolve tasks for a profile.
ResolveLibs is the only task that loads builtin libs and registers ambient modules.

### Prepare

Prepare prepares the per profile DIR for a module:
- Clone base DIR into a profile DIR.
- Build the export table from bound declarations and export statements.
- Do not resolve symbols or follow re-export chains.
- Requires `ResolveModulePrepare` on the current module.

### Direct

Direct resolves dependencies and builds the information required for canonicalization:
- Resolve dependency items and attach `target_symbol` where needed.
- Build the global symbol table (`declare global`, `export as namespace`) for the profile.
- Resolve `import` and `export` clauses **without** following re-export chains.
- Record `export *` edges **without** expanding them.
- Finalize export assignment and default targets from resolved expressions.
- Require `ResolveModulePrepare` on any referenced module.
- **Never** trigger another module’s Direct phase.

### Canonical

Canonical computes the canonical target for each symbol:
- Compute `canonical_symbol` for symbols that alias other symbols.
- Walk `target_symbol` chains across modules.
- Resolve re-export chains by consulting export tables.
- Yield if a referenced module is not ready.
- Requires `ResolveModuleDirect` on the current module.
- Requires `ResolveModulePrepare` on any module whose export table is consulted.

---

## Export Model

Every module owns an explicit export table keyed by `(SymbolSpace, StaticKey)`.
The export table is the only source of truth for cross-module lookup.

Export entries are either local or re-exported:

```ds
newtype Export =
    | { key: StaticKey, space: SymbolSpace, kind: "Local", symbol: LocalSymbolId }
    | { key: StaticKey, space: SymbolSpace, kind: "ReExport", item: LocalNodeId<DependencyItem> }
```

Export tables are populated by explicit exports and named re-exports:
- Declarations with explicit export modifiers
- `export { ... }` clauses
- `export default` clauses
- `export * as Name` clauses

Export tables are built during Prepare and remain stable across Direct.
Direct only fills target symbols for export assignments and default value exports.

`export * from "..."` is stored as a namespace export edge, not as direct entries.
Namespace export edges are kind-aware, so `export type *` does not participate in value lookups.

`export = value` is represented as an export assignment.
Export assignment is exclusive with other exports and does not create a named entry.
Import-equals bindings resolve against export assignments.

`SymbolSpace::TypeValue` exports produce both type and value entries.
Type-only and value-only exports remain in their own spaces.

**Default export key:** the export table uses the export name `default`.
Default exports conflict with any other export of the name `default`.
This matches ES module semantics and our `StaticKey::Name("default")` keying.

---

## Re-Exports and Cycles

Re-exports are resolved by walking export entries.
Re-export chains follow `ExportKind::ReExport` entries until a concrete "canonical" symbol is found.
`export *` is resolved by walking the target module’s export table at lookup time.
`export *` never re-exports the default export.
If multiple `export *` edges provide the same name, Resolve reports a conflict unless the targets match.

Example:

```ts
// a.ts
export { Foo } from "./b"

// b.ts
export { Foo } from "./c"
```

Resolve walks `a` → `b` → `c` and binds `Foo` to the final symbol.
Resolve reports a cycle if the chain returns to a visited module + name + space.
Cycles in re-export chains are errors:

```ts
// a.ts
export { Foo } from "./b"

// b.ts
export { Foo } from "./a" // error: re export cycle for Foo
```

Resolve reports a cycle for `Foo` and leaves the binding unresolved.

## Import Resolver

Resolve follows TypeScript’s module resolution order for every module import.
See [language/resolver](../../../resolver/README.md) for more details.
Resolution is order-sensitive, and resolution order matches TS semantics:

1. **Module declarations** (`declare module "name"`) are checked first.
2. **Builtin library aliases** are applied for builtin modules.
3. **Relative specifiers** are resolved against the source path.
4. **Bare specifiers** are resolved via the platform resolver.

Module declarations act as synthetic modules with their own scopes.
Module declarations are available in `.ts` and `.d.ts` files.
Module declarations from ambient libs are visible once ResolveLibs registers those libs.

Value imports resolve in the value space when possible.
If only a type export exists, the binding resolves to the type space.
Analyze enforces invalid value-position use of type-only imports.

Import-equals bindings (`import X = require("mod")`) use the same resolver and bind to export assignments.
Import-equals with a qualified name (`import X = A.B`) is lowered to a local alias and does not use module resolution.

`export as namespace Name` is supported only in declaration files.
Resolve treats it as a global augmentation that registers `Name` in the global scope.

`declare global { ... }` adds symbols to the global symbol cache.
Global symbols are indexed by `(StaticKey, SymbolSpace)` for fast lookups.
Global symbols are visible to all modules in the profile.

## Builtins

Builtin libs are registered as modules with explicit dependency graphs.
Builtin libs may be **ambient** or **explicit**.
Ambient libs participate in global symbol resolution.
Explicit libs must be imported or re-exported.

ResolveLibs is the only entry point that loads builtin libs and registers ambient modules.
Module resolution and module-binding lookup must treat builtin registration as read-only.
Any cross-module lookup should assume ResolveLibs already ran for the profile.

Builtin libs can define specifier aliases:

```ds
specifierAliases: [
    ("undici-types", "undici-types.v6")
]
```

Specifier aliases are resolved only for builtin modules.
Aliases allow versioned builtin packages without hardcoding names in Resolve.
