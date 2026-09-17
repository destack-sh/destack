---
title: Primitives
description: Number, yes, and int32, uint32, int64, float32, character.
---

# Primitives

TS++ has `number`, and adds explicit `float64`, `float32`, variable sized integers (`i<n>` for signed, `u<n>` for unsigned), pointer sized integers (`isize` / `usize`).
And of course we keep `string` and `bigint` with identical behavior, and additionally add `character`.

There was no real `symbol` use case left since we have nominality, so TS++ has no `symbol` or `unique symbol`.

## Numbers

Integer overflow and underflow traps (i.e. errors hard) in all build modes, and explicit wrapping / saturating / checked arithmetic is provided.

## Bigint
...


```ds:src/primitives.ds
const enabled: boolean = true;
const count: uint32 = 42;
const ratio: float64 = 0.75;
const initial: char = 'D';
const name: string = "Destack";
```

## Conversions

Conversions follow the same explicitness rule as the rest of TypeScript++: `as` between numeric types is allowed only when every value of the source type is representable in the destination (lossless), and lossy conversions must pick their behavior explicitly:

```ds
declare const wide: int32;

const a: int64 = wide as int64;   // OK: lossless widening
const b: uint8 = wide as uint8;   // ERROR: lossy conversion
const c = wide.truncate<uint8>(); // explicit: keep the low bits
const d = wide.saturate<uint8>(); // explicit: clamp into range
```

## Intervals

Ranges in type position become an interval type over _bounded_ sets like `int`, `bigint`, or `char`; basically, an interval type is a static subset of its scalar type (e.g., `1..4` in type position is equivalent to `1 | 2 | 3`).
Assignments to interval-typed places must already have an interval-compatible type - the compiler does _not_ prove arithmetic expressions stay inside intervals and we do not insert implicit runtime checks for interval assignments.

```ds
type Digit = 0..=9;
type LowerAscii = 'a'..='z';
type UserPort = 1024..=65535;

let digit: Digit = 7;
let letter: LowerAscii = 'm';
let port: UserPort = 8080;

digit satisfies int;
letter satisfies char;
port satisfies int;
```

## String

TS++ strings, like TS strings, follow JS `string` behavior: length and positional access are _defined_ in terms of UTF-16 code units, and - just like TS's own string iterator - iteration yields Unicode code points (mapping to `char`).

```ds
const text = "héllo";

text.length satisfies isize; // UTF-16 code units, as in JS

for (const c of text) {
    c satisfies char; // iteration by code point, as in JS
}
```

TS++ is stricter than TS for `string` indexing: `text[i]` gives a `char`, and indexing into a lone surrogate [traps](/docs/language/runtime/panics/).
On native targets, `string` is the library's `String` with owned UTF-16 code units; on JS/TS targets, strings use the host engine's representation.
