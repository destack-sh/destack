# expressions/

Destack extends TypeScript with expression-oriented control flow, a full pattern suite, Result-first error handling, and compile-time evaluation.
Almost all statements are expressions that produce values, and the last expression of a block becomes the block's value.

For example, classifying a parsed port in TypeScript means declaring a mutable binding and assigning to it through branches:

```ts
let status: string;
const parsed = parsePort(input);
if (parsed.ok && parsed.value >= 1024) {
    status = `user port ${parsed.value}`;
} else if (parsed.ok) {
    status = `system port ${parsed.value}`;
} else {
    status = parsed.error;
}
```

In Destack the match is the value: the Result variants destructure in place, the guard rides on the pattern, and there is nothing to assign:

```ds
const status = match (parsePort(input)) {
    Ok { value } if (value >= 1024) => `user port ${value}`
    Ok { value } => `system port ${value}`
    Err { error } => error
};
```

Patterns, guards, loops with break values, `using` cleanup, operator interfaces, and `comptime` evaluation all compose on the same principle.
The files below walk through them in order.

| file | shows |
| --- | --- |
| [`values.ds`](values.ds) | statements as expressions, implicit returns, `do` blocks, if-let |
| [`closures.ds`](closures.ds) | lexical capture with preserved variable identity |
| [`patterns.ds`](patterns.ds) | nominal, newtype, default, and must patterns, `let ... else` |
| [`guards.ds`](guards.ds) | precise `is` narrowing over unions and nominal types |
| [`loops.ds`](loops.ds) | `loop` with break values, range iteration and slicing |
| [`using.ds`](using.ds) | deterministic LIFO cleanup through the nominal `Dispose` interface |
| [`operators.ds`](operators.ds) | operator overloads as nominal interface implementations |
| [`arithmetic.ds`](arithmetic.ds) | trapping arithmetic, explicit wrapping/saturating/checked semantics |
| [`errors.ds`](errors.ds) | Result-first error handling, `?` propagation, matching variants |
| [`comptime.ds`](comptime.ds) | compile-time evaluation with results baked into the artifact |
