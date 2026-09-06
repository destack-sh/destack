---
title: Soundness
description: A module's public declarations can be recovered from that module alone.
---

# Soundness

- types are great, and powerful type systems are very useful.
<!--- I actually like the TS type system for the most part-->
- we just need to make it strict and sound, remove some footguns, and stabilize the ambiguities.
- and ofc need to be stable and deterministic, incrementally compilable, strict boundaries, make it fast

- So, how much of TS can we make sound, predictable, and fast?
- quite a lot, actually
- and it turns out that TS algebra and generics are much faster than macros since it's essentially a very constrained macro system already

- the easy part first to first: kill all the no soundness holes!
- no dynamic shenanigans, no JS legacy compat
- need strict, sound TS with predictable module boundaries and type behavior (roughly equivalent to tsc's `isolatedDeclarations`)
- "A module's public declarations can be recovered from that module alone."
- (isolated declarations must be directly transcribable)
- changing and constraining the type system just a bit gives us a lot more parallelism
- ideally also make it fast to compile, which requires cleaner boundaries than standard TS gives

All of the inherited JavaScript legacy-era dynamisms must go:
- no dynamic JS shenanigans or monkey patching, so goodbye to `__proto__` or anything like that
- no `module.x = foo..`, no mutable globals
- no `Object.prototype`, `Object.isOwnProperty`, `Object.assign`, ...
- no `Reflect.*`
- no `Proxy`
- no `delete obj.x`
- no `__proto__`
- no `eval` / `Function`
- no `with`
- no "truthiness"; conditionals always take booleans
- no array holes
- oh also: no sequence expressions, who needs sequence expressions

There are also some TypeScript features that are not sound or just not needed in a purely strict model:
- no declaration merging / no separate type and value spaces (as such)
- no `any`, no `as` except for widening casts
- no predicate functions (e.g. `isUser(user: any): asserts user is User` is unsound)
- unknown still works as a fat existential
- no symbol / string keyed duck typing (proper traits)
- no thenables (nominal Promise, TAsk only)
