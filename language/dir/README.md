# dir

Main Destack IR (DIR) definitions and program model.
DIR is the semantic intermediate representation that we do (almost) all of our semantic analysis on with all the classic stuff about symbols, scopes, and types.

## Overview

DIR is mapped from AST during binding (with a pretty close 1:1 correspondence), and then we do a _lot_ of in-place processing and transformation before ultimately delegating to Lower (MIR) or straight codegen (JS/TS).

```text
AST (syntax)  →  DIR (semantics)  →  MIR (machine)
    │                  │
  parser            compiler
```

DIR exists in two main forms:

**Base DIR** (profile-independent):
The Bind phase creates base DIR from AST:
- All syntactic sugar is desugared (`+=` → `+` and assign, `++` → `+= 1`, etc.)
- Symbols and scopes are declared
- But symbol references are not yet resolved, and types are not yet inferred

**Canonical DIR** (per-profile):
Resolve, Analyze, and Elaborate transform base DIR into canonical DIR for each profile. 
By the time DIR is "canonical" (fully elaborated), it has:
- All symbols resolved to their declarations (profile-dependent: library resolution depends on runtime/platform)
- All types inferred and checked
- All overloads resolved
- All patterns expanded to decision trees
- .. etc. see Elaborate

### Symbols and Scopes

Every named "thing" (variable, function, type, etc.) becomes a `Symbol` in the symbol table.
(And some unnamed things too, when we need to track them as symbols for various reasons.)
Symbols live in scopes, and scopes nest to form the lexical structure of the program.

```ds
struct Symbol {
    name: StringId,
    kind: SymbolKind,       // Namespace, Item, or Local
    space: SymbolSpace,     // Type, Value, TypeValue, or Label
    scope: LocalScopeId,    // where it's declared
    node: GlobalNodeIdAny,  // the declaring node
    // ...
}
```

TypeScript lets code have a "type `Foo`" and a "value `Foo`" in the same _scope_, so we also distinguish between SymbolSpaces.
Destack supports reflection and "types as values" semantics, so our types usually live in both type and value spaces.

### Types

The type system lives in its own table, separate from nodes.
Each type gets a `LocalTypeId`, and nodes reference types by ID rather than embedding them.

```ds
newtype Type =
    | TypeLiteral { value: TypeLiteral }     // primitives, never, any, etc.
    | Reference { symbol, staticArguments }  // named type with generics
    | Array { element }                       // T[]
    | Tuple { elements }                      // (T, U, V)
    | Function { params, returnType, ... }   // (T) => U
    // ... many more
```

Type inference happens in Analyze, which fills in the type table and links nodes to their types.

### Instances

An `Instance` is a concrete instantiation of a statically parameterized declaration.
When we write `Container<int32>`, that's an instance of the generic `Container<T>`, and Instances are how this is tracked.

```ds
struct Instance {
    symbolId: GlobalSymbolId,
    staticArguments: StaticArgument[],  // flattened: [inherited..., own...]
}
```

Arguments are stored flattened: inherited arguments from enclosing generic contexts come first, then own arguments.

```ds
struct Container<T> {
    map<U>(f: (T) => U): Container<U> { 
        ... 
    }
}

let c: Container<int32> = ...;
c.map<string>(f)
```

The instances created are:
- `Container<int32>` → `{ symbol: Container, arguments: [int32] }`
- `Container<int32>.map<string>` → `{ symbol: map, arguments: [int32, string] }`
For `map`, `int32` is inherited from `Container<T>` and `string` is `map`'s own `U`.

### Resolutions

A `Resolution` tells us how a **symbol lookup** was resolved at some usage site: every call, member access, and (non-builtin) operator.

```ds
newtype Resolution =
    | Unresolved { ... }              // couldn't find it
    | Builtin { receiver }            // primitive op, codegen handles it
    | Static { receiver, candidate }  // one target, known at compile time
    | Dynamic { receiver, candidates } // runtime dispatch needed (union types)
```

The different Resolution kinds come naturally from the specification and basically answer the question "what do we invoke here exactly?":
- **Builtin**: something the backend understands directly
- **Static**: direct call to a specific symbol
- **Dynamic**: may have multiple candidates, need runtime dispatch (like in a union)

For example, `a + b` where `a: int32` and `b: int32` resolves to `Builtin`, but `a.foo()` where `a: Cat | Dog` might resolve to `Dynamic` if `Cat::foo` and `Dog::foo` are different symbols.
Note that a polymorphic `T` might still resolve to `Static`: Resolution answers "what is the *symbol*?", not "how do we call it?".

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_dir
just language/test-specification
just language/test-emit

# clean gate
just language/quick

# exhaustive gate
just language/full
```
