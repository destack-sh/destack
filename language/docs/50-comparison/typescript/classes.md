---
title: Classes
description: Classes, compared to TypeScript.
---

# Classes

Classes in TS++ read like TS classes, and behave like them too: a class value is a handle to one shared object.
The differences are all about saying what you mean.

| TypeScript | TS++ |
| --- | --- |
| every method can be overridden | methods are final unless marked `virtual` |
| `override` is optional (`noImplicitOverride` makes it required) | `override` is required |
| `obj.method` reads a method, and `this` goes missing when called | reading a method off an object is an error |
| `private` is per class, `#field` is truly private | `private` is per module, there is no `#field` |
| fields may stay uninitialized with `!` | every field is initialized on every constructor path |

## Overriding

In TS any method can be replaced by a subclass.
In TS++ the base class decides which ones can.

```ts
class Shape {
    area(): number { return 0; }
}

class Square extends Shape {
    area(): number { return 4; }
}
```

```tspp
class Shape {
    virtual area(): float64 {
        0.0
    }
}

class Square extends Shape {
    override area(): float64 {
        4.0
    }
}
```

## Methods as Values

TS lets you read a method off an object, and the call then runs with the wrong `this`.
TS++ asks for the closure you meant.

```ts
const greet = greeter.greet; // compiles, `this` is undefined when called
```

```tspp
class Greeter {
    greet(): string {
        "hello"
    }
}

const greeter = new Greeter();
const greet = () => greeter.greet();
```

## Privacy

TS++ has one `private`, and it means "this module".
It compiles to `#field` on JS targets, so it's private at runtime too.

```ts
class Account {
    #balance = 0;
}
```

```tspp
class Account {
    private balance: int64 = 0;
}
```
