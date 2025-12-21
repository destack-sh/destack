# dir

Main Destack IR (DIR) definitions and program model.
DIR is the semantic intermediate representation: the IR that knows about symbols, scopes, and types.

## Overview

DIR sits between the AST and MIR in the pipeline.
The AST is purely syntactic ("what did you write?"), while DIR is semantic ("what does it mean?").

```
AST (syntax)  →  DIR (semantics)  →  MIR (machine)
    │                  │
  parser            compiler
```

The Bind phase creates DIR from AST, then Resolve, Analyze, and Elaborate progressively enrich, validate, and transform it.
By the time DIR is "canonical" (fully elaborated), it has:
- All syntactic sugar desugared (`+=` → `+` and assign, `++` → `+= 1`, etc.)
- All symbols resolved to their declarations
- All types inferred and checked
- All overloads resolved
- All patterns expanded to decision trees

Canonical DIR is the input to both JS/TS codegen (directly) and native codegen (via MIR lowering).
For JS targets, we map and print.
For native targets, we lower to MIR.

### Symbols and Scopes

Every named thing (variable, function, type, etc.) gets a `Symbol` in the symbol table.
(And some unnamed things we need to track as symbols for various reasons.)
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

TypeScript lets you have a type `Foo` and a value `Foo` in the same scope (interfaces do this) - we support this fully for `.ts` and `.tsx` files.
Destack supports reflection and "types as values" semantics, so types and values must occupy both the type and value namespaces.
In Destack `.ds` files, `Foo` as a type and `Foo` as a value must be the same thing.

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
When you write `Container<int32>`, that's an instance of the generic `Container<T>`.

```ds
struct Instance {
    symbolId: GlobalSymbolId,
    staticArguments: StaticArgument[],  // flattened: [inherited..., own...]
}
```

Arguments are stored flattened: inherited arguments from enclosing generic contexts come first, then own arguments.
This matches how Rust and C++ handle monomorphization: each instance is self-contained.

```
struct Container<T> {
    map<U>(f: (T) => U): Container<U> { ... }
}

let c: Container<int32> = ...;
c.map<string>(f)
```

The instances created are:
- `Container<int32>` → `{ symbol: Container, arguments: [int32] }`
- `Container<int32>.map<string>` → `{ symbol: map, arguments: [int32, string] }`

For `map`, `int32` is inherited from `Container<T>` and `string` is `map`'s own `U`.

### Resolutions

A `Resolution` tells you how a **symbol lookup** was resolved at some usage site.
This is what the Analyze phase produces for every call, member access, and operator.

```ds
newtype Resolution =
    | Unresolved { ... }              // couldn't find it
    | Builtin { receiver }            // primitive op, codegen handles it
    | Static { receiver, candidate }  // one target, known at compile time
    | Dynamic { receiver, candidates } // runtime dispatch needed (union types)
```

The distinction matters for codegen:
- **Builtin**: emit a primitive instruction (`iadd`, `fcmp`, etc.)
- **Static**: emit a direct call to the resolved symbol
- **Dynamic**: emit a type switch to dispatch between candidates at runtime

For example, `a + b` where `a: int32` and `b: int32` resolves to `Builtin`.
But `a.foo()` where `a: Cat | Dog` might resolve to `Dynamic` if `Cat::foo` and `Dog::foo` are different symbols.
Polymorphic types might still resolve to `Static` even with vtable lookup: Resolution answers "what is the *symbol*?", not "how do we call it?".
If we don't know the symbol at compile time, it's dynamic dispatch.