# Satisfies Operator

The satisfies operator applies contextual typing while preserving the expression type.
It does not widen away useful literal information when a const context is present.

## Contextual typing

### satisfies provides contextual typing for lambdas

> Lambdas inside satisfies is contextually typed by the target type.

```ds
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
```

### satisfies rejects incompatible lambda calls

> Contextual typing still rejects incompatible calls.

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

> Const bindings preserve literal members when satisfies provides the context.

```ds
type Mode = "dev" | "prod";

const config = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";
```

### satisfies enforces excess property checks

> Object literals still undergo excess property checks under satisfies.

```ts
type Shape = { a: number };

const value = { a: 1, b: 2 } satisfies Shape;
```

- contains: excess property

### satisfies does not widen without const context

> Satisfies provides contextual typing without forcing const contexts.

```ts
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "dev";
```

### satisfies contextual members reject unrelated literals

> Contextual member inference rejects unrelated literals.

```ts
type Shape = { mode: "dev" | "prod" };

let config = { mode: "dev" } satisfies Shape;
config.mode satisfies "prod";
```

- contains: not assignable

## expression identity

### satisfies keeps source members after validation

> Satisfies validates against the target while keeping source member access.

```ts
type Target = { mode: "dev" | "prod"; retries: number };

const config = { mode: "dev", retries: 3 } satisfies Target;

config.retries satisfies number;
```

### satisfies keeps source method signatures

> Satisfies does not erase source method signatures after target validation.

```ts
type Target = { mode: "dev" | "prod" };

const config = {
    mode: "dev",
    next(value: number) {
        return value + 1;
    },
} satisfies Target & { next(value: number): number };

config.next(1) satisfies number;
```

## assignment target behavior

### satisfies expressions are not assignment targets

> Satisfies expressions are not legal assignment targets.

```ts
let value = 1;
(value satisfies number) = 2;
```

- invalid assignment target
