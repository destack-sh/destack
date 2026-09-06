---
title: Async and Promise
description: keep familiar Promise for aliased async
---

# Async and Promise

- proper async
- keep familiar Promise for aliased async
- Promise is implemented basically completely in userland!
- *fiber*-based execution (e.g. JVM's new Loom model).. but doesn't really matter, feels like TS
- (for soundness, Promis requires Copy values, which classes and primitive value types trivially satisfy)

- TS++ has no exceptions, promises never reject quite like they do in TS++
- (they're really more like Futures once you remove the exception model)
- `Promise<Result<T, E>>` as the result type for fallible async work
- (or `Task<Result<T, E>>` for affine execution)
