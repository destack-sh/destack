---
title: Strict
description: Sound *and* what you meant.
---

# Strict

Some TS code is perfectly sound and still almost always a mistake, or, just _probably_ not quite what you wanted.
In such potentially ambiguous situations, TS++ makes you say what you mean:
 1. Conditions take a `boolean`, not whatever happens to be truthy.
 2. Methods don't come off their objects (and lose `this` on the way).
 3. Overriding a method is explicit, on both sides (`virtual`, `override`).
 4. Every field must have a value by the time it is used.

## Conditions Are Booleans

An empty string or a `0` never quietly counts as false; you say what you mean instead:

```tspp
declare const name: string;

if (name) {
//  ^^^^ error[non-boolean-condition]: condition must be boolean, found 'string'
}

if (name.length > 0) {
}
```

## Methods Stay on Their Objects

In TS, reading `greeter.greet` gives you a function that forgets its `this`.
TS++ asks for the closure you meant:

```tspp
class Greeter {
    greet(): string {
        "hello"
    }
}

const greeter = new Greeter();
const greet = () => greeter.greet();
const detached = greeter.greet;
//                       ^^^^^ error[cannot-extract-bound-method]: method 'greet' cannot be read as a value
```

## Overrides Are Explicit

A base class marks what can be overridden with `virtual`, and a subclass says `override` when it replaces it, so an override never happens by accident:

```tspp
class Shape {
    virtual area(): float64 {
        0.0
    }
}

class Circle extends Shape {
    override perimeter(): float64 {
//           ^^^^^^^^^ error[invalid-override]: 'perimeter' does not override an inherited member
        0.0
    }
}
```

## Fields Start With a Value

Every field gets a value from its initializer or the constructor, on every path (there's no `!` to promise it'll be fine later):

```tspp
class User {
    name: string;
    visits: int32;
//  ^^^^^^ error[field-not-definitely-initialized]: field 'visits' is not initialized on every constructor path

    constructor(name: string) {
        this.name = name;
    }
}
```
