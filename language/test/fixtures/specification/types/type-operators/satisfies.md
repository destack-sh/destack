# Satisfies Operator

The satisfies operator applies contextual typing while preserving the expression type.
It does not widen away useful literal information when a const context is present.

## contextual typing

### satisfies provides contextual typing for lambdas

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```

### satisfies rejects incompatible lambda calls

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

```ds
type Mode = "dev" | "prod";

const config = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";
```

### satisfies enforces excess property checks

```ds
type Shape = { a: number };

const value = { a: 1, b: 2 } satisfies Shape;
```

- contains: excess property

### satisfies does not widen without const context

```ds
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "dev";
```

### satisfies contextual members reject unrelated literals

```ds
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "prod";
```

- contains: not assignable

## expression identity

### satisfies keeps source members after validation

```ds
type Target = { mode: "dev" | "prod"; retries: number };

const config = { mode: "dev", retries: 3 } satisfies Target;

config.retries satisfies number;
```

### satisfies keeps source method signatures

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

```ds
let value = 1;
(value satisfies number) = 2;
```

- contains: invalid assignment target
