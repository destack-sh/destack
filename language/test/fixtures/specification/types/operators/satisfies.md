# Satisfies Operator

The satisfies operator should apply contextual typing while preserving the expression type.
It should not widen away useful literal information when a const context is present.

## Contextual typing

### satisfies provides contextual typing for lambdas

> Lambdas inside satisfies should be contextually typed by the target type.

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```

### satisfies rejects incompatible lambda calls

> Contextual typing should still reject incompatible calls.

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run("no");
```

- contains: not assignable

## Literal preservation

### satisfies preserves literal members under const bindings

> Const bindings should preserve literal members when satisfies provides the context.

```ds
type Mode = "dev" | "prod";

const config = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";
```
