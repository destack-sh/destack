---
title: Classes
description: Classes are generally pretty straightforward.
---

# Classes

- classes are generally pretty straightforward, it's just about which tradeoffs do we want?
- about a dozen way of doing classes, from decent to okay to weird JVM? C++? Go?
- "zero overhead"? vtable pointers? explicit or implicit virtual?
- allocation metadata sidetable on the heap / runtime
- classes are reference types by default, alias freely
- abstract, final classes
- virtual, override methods

- in TS, like in many managed languages, we can just omit the "self" parameter in a method and the receiver will just default to the aliasing managed reference "this"
- we support this ofc as well:
- `this` = `Managed<T>` for reference types
- implicit and explicit this
- value and borrowed forms

- no method binding (i.e. no sneaky obj.method, instead use () => object.method()) for clarity)

- visibility:
- default is public (as in TS)
- private/protected is per module, not per item
- no need for #privateField
- we have private, we just use that
- and it codegens to #privateField on JS targets
- no additional visibiliity controls

```ds:src/counter.ds
class Counter {
    value = 0;

    increment(): void {
        this.value += 1;
    }
}
```

## Static Members

- static in terms of static _association_, evaluated at runtime
- module level constants
- static members and evaluation order
- static members per instance / specialisation
