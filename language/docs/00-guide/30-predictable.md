---
title: Predictable
description: Code that does what it says, the same way, every time.
---

# Predictable

TS++ tries hard to make code do exactly what it looks like it does:
 1. Numbers have sizes, and literals have to fit.
 2. Overflow stops the program, and conversions never silently drop bits (unless you ask them to).
 3. Objects have exactly the fields their type says.
 4. One name, one declaration: no declaration merging.
 5. Declarations write their types, so you can read a module without reading its bodies.

## Numbers Have Sizes

There's still `number`, and next to it `int32`, `uint8`, `float64` and friends, so you know exactly what fits:

```tspp
const count: int32 = 42;
const ratio: float64 = 0.75;
const small: uint8 = 300;
//                   ^^^ error[not-assignable]: type '300' is not assignable to type 'uint8'
```

## Overflow Is Loud

Integer overflow traps in every build mode.
When you actually want wrapping or a check, you say so:

```tspp
function next(counter: uint8): uint8 {
    counter.wrappingAdd(1)
}

function total(left: uint8, right: uint8): uint8 | undefined {
    left.checkedAdd(right)
}
```

Converting between number types works the same way: `as` only widens, and narrowing picks a behavior.

```tspp
declare const wide: int32;

const widened = wide as int64;
const narrowed = wide as uint8;
//               ^^^^ error[invalid-cast]: type 'int32' cannot be cast to 'uint8'
const low = wide.truncate<uint8>();
const clamped = wide.saturate<uint8>();
```

## Objects Have Their Fields

An object type is exact: a value has the fields its type lists, no more and no less.
(Note: in TS, an object with extra fields quietly passes as a smaller type; in TS++ it doesn't, which is also what gives objects a fixed layout, see [Fast](50-fast.md).)

```tspp
type Point = { x: float64; y: float64 };

function length(point: Point): float64 {
    point.x * point.x + point.y * point.y
}

function main(): float64 {
    const point = { x: 1.0, y: 2.0, z: 3.0 };
    length(point)
//         ^^^^^ error[argument-not-assignable]: argument of type '{ x: float64; y: float64; z: float64 }' is not assignable to parameter of type 'Point'
}
```

## One Name, One Declaration

TS merges declarations that share a name: two `interface User`s become one, a namespace can extend a function.
TS++ doesn't, so what you see where something is declared is all there is:

```tspp
interface User {
    name: string;
}

interface User {
    age: int32;
}

function describe(user: User): int32 {
//                      ^^^^ error[ambiguous-reference]: ambiguous reference 'User'
    user.age
}
```

## Declarations Write Their Types

Function declarations write their result type and exports write their type, so a module's shape reads without its bodies (and checks without them too, which keeps builds fast).
(Note: this is basically `isolatedDeclarations` from TS, just always on.)
Local closures still infer their types, as you'd expect:

```tspp
function double(value: int32) {
//       ^^^^^^ error[missing-result-type]: function declaration needs a written result type
    value * 2
}

export const triple = (value: int32) => value * 3;
//           ^^^^^^ error[missing-export-binding-type]: exported binding needs a written type

function quadruple(value: int32): int32 {
    const twice = (value: int32) => value * 2;
    twice(twice(value))
}
```
