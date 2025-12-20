# MIR Optimization

This document describes the optimization passes applied to MIR before code generation.
These passes run after Lower and before Generate, transforming MIR in-place for better performance.

## Overview

```
Lower → MIR → Optimize → Generate
              ├─ Escape Analysis
              ├─ Inlining
              ├─ Devirtualization
              ├─ Constant Propagation
              ├─ Dead Code Elimination
              └─ ... (Cranelift does most of the rest)
```

For now, we rely heavily on Cranelift for low-level optimizations (register allocation, instruction selection, peephole opts).
Our passes focus on high-level semantic optimizations that Cranelift can't do.

## Inlining

Inlining is the most important optimization.
It eliminates call overhead and enables further optimizations (escape analysis, constant propagation, dead code elimination).

### Inlining Heuristics

| Factor | Weight | Notes |
|--------|--------|-------|
| Call site hotness | High | Profile-guided or static estimate |
| Callee size | Medium | Small functions preferred |
| Call site arguments | Medium | Constant arguments enable specialization |
| Recursion depth | Negative | Limit recursive inlining |
| `@inline` hint | High | User-requested inlining |
| `@noinline` hint | Blocks | User-requested no inlining |

### Inlining Across Generics

After monomorphization, generic functions become concrete and inline like any other function:

```
// generic source
function identity<T>(x: T): T { x }
const n = identity(42)

// after monomorphization
function identity_int32(x: int32): int32 { x }
const n = identity_int32(42)

// after inlining
const n = 42
```

### Inlining and Closures

Inlining closure calls requires care:
- Lambda body can be inlined at call site
- Captures must be correctly substituted
- Environment struct may be eliminated if all uses are inlined

## Escape Analysis

Escape analysis determines whether objects can be stack-allocated instead of heap-allocated.
This is critical for performance: stack allocation is essentially free, while heap allocation requires GC tracking.

### What Escapes

A value **escapes** if it might be accessed after the current function returns:

```
function escapes(): User {
    const u = User { name: "Alice" }
    return u                          // escapes: returned to caller
}

function alsoEscapes(list: User[]) {
    const u = User { name: "Bob" }
    list.push(u)                      // escapes: stored in external collection
}

function doesNotEscape() {
    const u = User { name: "Carol" }
    print(u.name)                     // does not escape: only local use
}                                     // u can be stack-allocated
```

### Escape Categories

| Category | Escapes? | Allocation |
|----------|----------|------------|
| Returned from function | Yes | Heap |
| Stored in heap object | Yes | Heap |
| Passed to unknown function | Maybe | Heap (conservative) |
| Passed to known non-capturing function | No | Stack |
| Captured by escaping closure | Yes | Heap |
| Captured by non-escaping closure | No | Stack |
| Only local reads/writes | No | Stack |

### Analysis Implementation

Escape analysis runs after inlining for maximum effectiveness.
For each allocation site:

1. Track all uses of the allocated value
2. Check if any use causes escape (return, store to heap, capture)
3. If no escapes, emit `StackAlloc` instead of `ManagedAlloc`

```mir
; Source: function process() { const temp = Point { x: 1, y: 2 }; return temp.x + temp.y }

; With escape analysis: temp doesn't escape, use stack
v0 = stack.alloc Point
field.set v0, 0, 1    ; x
field.set v0, 1, 2    ; y
v1 = field.get v0, 0
v2 = field.get v0, 1
v3 = iadd v1, v2
return v3
; v0 automatically freed when function returns
```

### Closure Capture Analysis

Closures complicate escape analysis.
A closure captures variables from its environment, and if the closure escapes, so do its captures.

```
function makeAdder(x: int): (int) => int {
    return (y) => x + y    // closure escapes, so x must be heap-allocated
}

function localClosure() {
    const x = 10
    const add = (y: int) => x + y    // closure doesn't escape
    print(add(5))                     // x can be stack-allocated
}
```

We analyze closure escape at the call site:
- Closure passed to `Array.map` and result used locally → doesn't escape
- Closure stored in field or returned → escapes

### Conservative Fallback

When analysis can't determine escape (e.g., value passed to external function), we conservatively heap-allocate.
This maintains correctness at the cost of some performance.

For hot paths where manual control is needed, use explicit `^T` (value) types:

```
function hotPath(^point: Point) {    // force value semantics
    // point is always passed by value, never heap-allocated
}
```

## Devirtualization

When dynamic vtable dispatch can be resolved to a single implementation, we devirtualize to direct calls.

### Sources of Devirtualization

1. **Sealed types**: Class with no subclasses in the compilation unit
2. **Final methods**: Methods that can't be overridden
3. **Monomorphized generics**: After monomorphization, concrete types are known
4. **Flow analysis**: Type narrowing proves single implementation

```
// Before devirtualization
interface Drawable { draw(): void }
function render(d: Drawable) { d.draw() }  // vtable lookup

// After (if only Circle implements Drawable in scope)
function render(d: Drawable) { Circle_draw(d) }  // direct call
```

### Speculative Devirtualization

For polymorphic call sites with a dominant receiver type, emit guarded direct call:

```mir
; Polymorphic call with 90% Circle, 10% Square
v1 = field.get v0, 0           ; get type_id
branch (v1 == CIRCLE_TYPE_ID), block_fast, block_slow

block_fast:
    call @Circle_draw(v0)      ; fast path: direct call
    jump block_merge

block_slow:
    ; slow path: vtable lookup
    v2 = field.get v0, 1       ; vtable ptr
    v3 = element.get v2, 0     ; draw method
    call_indirect v3(v0)
    jump block_merge

block_merge:
    ; continue
```

## Constant Propagation

Replace uses of variables with their constant values when known.

```mir
; Before
v0 = iconst 10i32
v1 = iconst 20i32
v2 = iadd v0, v1

; After
v2 = iconst 30i32
```

This enables further dead code elimination (v0 and v1 are now unused).

## Dead Code Elimination

Remove instructions whose results are never used.

```mir
; Before
v0 = iconst 10i32    ; unused
v1 = iconst 20i32
return v1

; After
v1 = iconst 20i32
return v1
```

## Future Optimizations

We plan to add:
- **Loop optimizations**: unrolling, invariant code motion, strength reduction
- **Alias analysis**: better understanding of pointer relationships
- **Profile-guided optimization**: use runtime profiles to guide decisions
- **Cross-module inlining**: inline across compilation units with LTO

For most low-level optimizations, we rely on Cranelift's excellent optimization passes.
