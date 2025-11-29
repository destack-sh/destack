# Destack Language Design

Destack is TypeScript extended for building correct, optimal, integrated full-stack systems.
Valid JavaScript is valid Destack.
Valid TypeScript is valid Destack.

## The Vision

We're very early in software as an industry.
We could do so much more—software is embarassingly broken and slow.
We want to make correct, optimal, integrated full-stack software systems simple and fast to build.
But that requires unifying all the disparate pieces together: one language, one type system, one way of thinking about code from UI to servers to simulations.

TypeScript is the closest thing we have to that foundation today.
It runs everywhere, everyone knows it, and it has a massive ecosystem.
Destack builds on TypeScript with extensions that wouldn't fit in TypeScript itself (like `.tsx` or `.svelte`) without splintering the ecosystem.

## Higher-Order Software

TypeScript is a language for describing *what data looks like*.
Destack extends it to describe *how software behaves*: what effects it has, what constraints it requires, what patterns it matches, how it should be optimized, how it can be debugged.

Software is more than code.
It's documentation, tests, lints, telemetry, debugging, deployment.
These pieces are usually scattered across different tools with different languages and different mental models.
Destack aims to bring them together—not by replacing specialized tools, but by giving them a shared foundation.

This is what "higher-order software" means: raising the bar on what we can express, verify, and automate.

### What We Can Express

Instead of just types for data, Destack lets you express:

- **What a function needs**: effect declarations (`with FileSystem, Network`)
- **What a type guarantees**: constraints (`where T: Serializable`)
- **What structure data has**: precise types, structs, pattern matching
- **What operations mean**: operator overloading, extension methods
- **What code produces**: everything is an expression with a value
- **What matters**: structured annotations (`#tags`, `@decorators`)

The more we can express in the language, the more the toolchain can verify, optimize, and assist.
Custom type-aware lints become possible.
Visual debugging can show actual data structures.
Documentation stays in sync because it's part of the code.

## TSX Everywhere

TSX brought declarative tree syntax to React.
But that syntax is useful far beyond UI:

- **Entity trees** for games and simulations
- **Prompt structures** for AI interactions
- **Document layouts** for content systems
- **Scene graphs** for 3D applications
- **Configuration** as typed, validated structures

Destack generalizes TSX so `<Node>` syntax works with any tree-shaped data, not just React components.
Same familiar syntax, broader applicability.
And all in the same `.ds` files, all working with the existing `.ts` ecosystem.

## Types as Values

In TypeScript, types exist only at compile time—they're erased before runtime.
This limits what you can do: no runtime type checking, no serialization based on types, no generic factories.

Destack makes types available as values.
A type annotation isn't just a hint to the compiler—it's data you can inspect, pass around, and use at runtime.
This enables:

- Runtime validation against declared types
- Automatic serialization/deserialization
- Generic factories that know their type parameters
- Reflection without separate metadata systems

## Expressions, Not Statements

In TypeScript, `if` is a statement.
You can't assign its result to a variable without a ternary or temporary.
Same for `switch`, loops, blocks.

In Destack, everything is an expression:

```
const result = if condition { computeA() } else { computeB() }
const label = match state {
    Ready => "go"
    Loading => "wait"
    Error(e) => `failed: ${e}`
}
```

As everything in Destack, this is optional, but it changes how you can structure code.
Fewer temporaries, more inline logic, clearer data flow, more clarity.

## Precision When You Need It

TypeScript's `number` is always a 64-bit float.
That's usually surprisingly fine, but sometimes you want:

- Exact integer semantics for IDs, indices, counts
- Specific bit widths for binary protocols
- Overflow behavior you can reason about
- Types that can eventually compile to efficient native code

Destack adds precise types (`int32`, `uint64`, `float32`) as opt-in alternatives.
Use `number` when it doesn't matter, precise types when it does.

## Describing Behavior

Effects, constraints, and patterns let you describe *how* code behaves, not just what types it uses:

**Effects** declare what a function can do:
```
function saveData(data: Data) with FileSystem, Network { ... }
```

**Constraints** specify what types must support:
```
function serialize<T>(value: T): string where T: Serializable { ... }
```

**Patterns** destructure and match exhaustively:
```
match response {
    Ok(data) => process(data)
    Err(e) if e.retryable => retry()
    Err(e) => fail(e)
}
```

These aren't just type system features—they're ways of making implicit assumptions explicit and verifiable.

## The Integrated Stack

A language is most powerful when it's integrated with its toolchain and ecosystem.
Destack is designed to work with:

- **Unified types** across frontend and backend
- **Shared validation** that runs anywhere
- **Consistent patterns** from UI to database
- **Single debugging** experience across the stack
- **Type-aware tooling** that understands your domain

The language extensions enable tighter integration: effects can be tracked across service boundaries, types can be serialized consistently, patterns can be shared between client and server.

## The TSX Model (Again)

TSX didn't replace TypeScript—it extended it.
The ecosystem embraced it because:

- Valid TypeScript is valid TSX
- `.ts` and `.tsx` files coexist
- It compiles to standard JavaScript
- It adds expressiveness without breaking anything

Destack follows the same model.
It's not a fork, not a competitor—it's an extension.
Your TypeScript knowledge transfers.
Your npm packages work.
Your tooling functions.
You adopt incrementally, feature by feature, file by file.

## One Ecosystem

This matters enough to say twice: Destack is not trying to fragment the JavaScript/TypeScript ecosystem.
It's trying to expand what that ecosystem can do.

- **Destack → JS/TS**: Compiles to readable JavaScript
- **JS/TS → Destack**: Import any npm package
- **Mixed codebases**: `.ts` and `.ds` interoperate seamlessly

The `.d.ds` extension works like `.d.ts`—you can add Destack type information to existing packages without modifying them.

Valid TypeScript is valid Destack.
That's the commitment.

## Design Principles

**Familiar before novel**: TypeScript patterns where possible.
Rust/Kotlin/Swift patterns where TypeScript lacks.
New syntax only when it enables something important.

**Explicit over implicit**: Make behavior visible.
Effects declared, constraints stated, patterns exhaustive.
No hidden magic (..as little as possible).

**Gradual complexity**: Simple code stays simple.
Advanced features are opt-in.
You only pay for what you use.

**Expressions over statements**: Code that computes values, not code that mutates state.
Functional patterns made natural without forcing them on you.