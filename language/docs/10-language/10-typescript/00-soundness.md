---
title: Soundness
description: A module's public declarations can be recovered from that module alone.
---

# Soundness

Types are great, and powerful type systems are very useful since they let us express lots of invariants that are then checked by the compiler before anything even runs.
TS already has a powerful type system, we just need to make it strict and sound, remove some footguns, and stabilize the ambiguities.

## Soundness

There are well known soundness holes and stability issues in TypeScript, so let's just get rid of them:
- No circular _inference_ across modules (like a stronger `isolatedDeclarations`)
- No `any`, no `as` except for widening casts
- No dynamic JS shenanigans or monkey patching
- No `__proto__` or anything like that
- No `module.x = foo..`
- No `Object.prototype`, `Object.isOwnProperty`, `Object.assign`, ...
- No `Reflect.*`
- No `Proxy`
- No `delete <obj.x>`
- No `__proto__`
- No `eval` / `Function`
- No `with`
- No predicate functions (e.g. `isUser(user: any): asserts user is User` is unsound)
- No "truthiness" (conditionals always take `boolean`s)
- No array holes

## Superseded

There are also some TypeScript features that are technically sound but just not needed in a purely strict, nominal, modern language:
- No sequence expressions (who needs sequence expressions?)
- No declaration merging / no separate type and value spaces (as such)
- No symbol / string keyed duck typing (proper traits)
- No thenables (nominal `Promise`, affine `Task`, nominal `Awaitable` interface)
