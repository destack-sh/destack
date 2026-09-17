---
title: Results and Try
description: Errors as values.
---

# Results and Try

Most changes from TS to TS++ are about soundness and completeness, but there is nothing intrinsically unsound about exceptions.
- if there is one really bad error in modern managed languages, it's exceptions
- this one is a little more subjective, but given the pain caused.. exceptions most die. we cannot have an invisible side channel infecting everything in the last computing stack
- checked exceptions are even worse

- honestly, nobody has figured out a *great* way to do error handling (looking at Go here in particular), but Swift and Rust's Result-shaped error values with Try operators `?` are pretty good
- panic/unwind still exists like in rust but that's worker-scoped, not normal recovery
- Try operator, ? ambiguity because TS
- but exceptions have proven troubling over and over and over again
- checked exceptions are even worse
- the only sane error handling method is the Swift-y Rust-y ? operator

- result and async (promise / task)
- host promises that can reject are adopted into result carriers, or converted into panics, at the binding boundary

- Try-Catch-Finally still works
- try catch finally is ofc also an expression and closes on its value
- familiar try / catch / finally syntax still works though!
- catch (e) is all Try error residuals
- catch match (e) as the ergonomic switch

## Maybe, Must and Coalesce

`?`, postfix `!`, and `??` all unwrap the same `Try`-based absence-or-failure shape.
Opening removes outer `null` / `undefined`, opens one `Try` carrier, and removes `null` / `undefined` from the carrier's success value.
It's much simpler than it sounds:

```ds
declare const x: Result<T | null | undefined, E | null | undefined> | null | undefined;

// x?
// -> success: T
// -> failure (propagated): E | null | undefined
```

Nullish values on the failure side remain in the failure side.
The three operators differ on the failure case:

```ds
x?      // success T, failure leaves the expression
x!      // success T, failure traps
x ?? y  // success T, failure evaluates y
```

The try operator `?` keeps the success value and lets absence or failure leave the current expression in whichever way the container requires.
Inside a `try` block with `catch`, propagation transfers the failure value to the catch instead.

```ds
function readConfig(path: string): Result<Config, IOError | ParseError> {
    const text = readFile(path)?;
    const json = parseJson(text)?;
    return Result.ok(Config.from(json));
}
```

The try-coalesce operator `??` accepts the same shape locally "within" the expression with a direct fallback instead of letting it bubble up to the container as with `?`.
The result of `Result<T, E> ?? F` is the non-nullish opened success type joined with the fallback type `T | F`:

```ds
declare const defaultConfig: Config;

declare function loadConfig(): Result<Config, IOError> | null;
const a = loadConfig() ?? defaultConfig;
a satisfies Config;

declare function loadMaybeConfig(): Result<Config | null | undefined, IOError | null> | undefined;
const b = loadMaybeConfig() ?? defaultConfig;
b satisfies Config;
```

All unwrap operators unwrap exactly _one_ layer of `Try`, so nested `Try` values inside the success type also stay wrapped at the inner layer:

```ds
declare function loadNested(): Result<Result<Config, ParseError>, IOError>;

const c = loadNested() ?? defaultConfig;
c satisfies Result<Config, ParseError> | Config;
```

Postfix `!` is the "must" forced unwrap form: it opens the same outer nullish and single `Try` layer, but [traps](/docs/language/runtime/panics/) instead of propagating or falling back when the value is absent or failed:

```ds
const config = loadConfig()!;
config satisfies Config;
```

Regarding precedence, whitespace decides between the try operator and a ternary, which should mostly follow how we would naturally type (and format) these expressions anyway: an attached question mark is try-propagation, a detached one is a ternary condition.
This keeps the branches unambiguous in both directions: `flag ? -x : x` is a conditional, and `x? - 1` subtracts from the opened success value (so a compact ternary requires its spaces in `.ds`).
