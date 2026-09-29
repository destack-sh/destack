---
title: Sound
description: Types that can't lie.
---

# Sound

In TS, types are a best effort: `any`, unchecked casts and stale narrowing can make a type lie, and the program only finds out at runtime (if at all).
TS++ compiles, so types have to hold, always:
 1. There is no `any`.
 2. `as` can't lie.
 3. Narrowing stays true, even when something else changes the value.

## No `any`

`any` switches the checker off, so TS++ doesn't have it.
When you really don't know what something is, use `unknown` and narrow it, just like in TS:

```tspp
let value: any = 1;
//         ^^^ error[unresolved-reference]: cannot find 'any'

function describe(value: unknown): string {
    if (value is string) {
        value
    } else {
        "something else"
    }
}
```

## `as` Can't Lie

`as` only converts when the conversion always holds, so it can't claim an `unknown` is a `string`, or that any `Shape` is a `Square`.
Narrow instead, and the checker knows it's true:

```tspp
class Shape {}
class Square extends Shape {}

declare const value: unknown;
declare const shape: Shape;

const text = value as string;
//           ^^^^^ error[invalid-cast]: type 'unknown' cannot be cast to 'string'
const square = shape as Square;
//             ^^^^^ error[invalid-cast]: type 'Shape' cannot be cast to 'Square'
```

## Narrowing Stays True

In TS, a narrowed field stays narrowed across a call, even when the call changes the field.
TS++ ends the narrowing of a field at any call, `await`, `yield`, or write through a reference, since any of them can change it.
To keep a narrowing, narrow a local copy instead:

```tspp
class Holder {
    name: string | undefined = undefined;
}

function reset(holder: Holder): void {
    holder.name = undefined;
}

function stale(holder: Holder): string {
    if (holder.name != undefined) {
        reset(holder);
        holder.name
//      ^^^^^^^^^^^ error[stale-narrowing]: 'holder.name' may have changed since it was narrowed
    } else {
        "nobody"
    }
}

function copied(holder: Holder): string {
    const name = holder.name;
    if (name != undefined) {
        reset(holder);
        name
    } else {
        "nobody"
    }
}
```
