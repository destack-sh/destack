---
title: Captures
description: Control captures.
---

# Captures

- capture
- the default is "managed" / automatic as in TS, which means we don't have to think about captures, but incur some allocation cost

```ds
function offset(amount: int32): (value: int32) => int32 {
    return (value) => value + amount;
}
```

## Capture Modes

- on demand when desired we can specify `@capture` for lambdas / nested functions
- `@capture` with `"move"`, `"borrow"`, `"copy"`,`"manage"`, ..
- `@capture("manage")` preserves identity, `@capture("borrow")` borrows the original binding, `@capture("copy")` snapshots it, and `@capture("move")` transfers it

```ds
@capture({
    default: "copy",
    socket: "move",
    logger: "borrow",
    this: "borrow",
})
return (message) => {
    logger.info("sending");
    return socket.write(`${this.prefix}: ${message}`);
};
```

- `"repeatable"` is the default multiplicity, while `"once"` is affine and consumed by its first invocation
- closures preserve lexical `this` and use `Function<Parameters, Return, Multiplicity>`;

- lambdas (fat pointers with env)
- `Function`, `^Function`, and `&Function` use managed, owned, and borrowed environments
- repeatable calls require `&Function` or stronger access and preserve the environment
- `&readonly Function` cannot be called; `&Function` grants the required mutable access
- only `^Function<Parameters, Return, "once">` is valid; its call consumes the callable
- a closure / coroutine carries the lifetime of every borrow it captures on its type (like a Rust closure or `Future + 'a`)
