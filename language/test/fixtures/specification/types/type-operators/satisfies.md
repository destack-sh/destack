# Satisfies Operator

The satisfies operator applies contextual typing while preserving the expression type.
It does not widen away useful literal information when a const context is present.

## contextual typing

### satisfies provides contextual typing for lambdas

The target type types the lambda parameters.

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```

### satisfies rejects incompatible lambda calls

Contextual parameters keep their target types.

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run("no");
```

- contains: not assignable

## literal preservation

### satisfies preserves literal members under const bindings

Validation does not widen the source.

```ds
type Mode = "dev" | "prod";

const config = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";
```

### satisfies enforces excess property checks

Fresh literals stay exact.

```ds
type Shape = { a: number };

const value = { a: 1, b: 2 } satisfies Shape;
```

- contains: excess property

### satisfies keeps contextual literal members

The source type survives even under `let`.

```ds
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "dev";
```

### satisfies contextual members reject unrelated literals

Keeping the source type cuts both ways.

```ds
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "prod";
```

- contains: not assignable

## expression identity

### satisfies keeps source members after validation

Members beyond the checked shape remain visible.

```ds
type Target = { mode: "dev" | "prod"; retries: number };

const config = { mode: "dev", retries: 3 } satisfies Target;

config.retries satisfies number;
```

### satisfies keeps source method signatures

Method signatures stay the source's.

```ds
type Target = { mode: "dev" | "prod" };

const config = {
    mode: "dev",
    next(value: number) {
        return value + 1;
    },
} satisfies Target & { next(value: number): number };

config.next(1) satisfies number;
```

## assignment targets

### satisfies expressions are not assignment targets

`satisfies` produces a value, not a place.

```ds
let value = 1;
(value satisfies number) = 2;
```

- contains: invalid assignment target
