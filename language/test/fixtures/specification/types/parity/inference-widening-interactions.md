# Inference And Widening Parity

## nested generic inference

### nested callback forwarding preserves const tuple element precision

Forwarding const tuples through nested higher-order callbacks should preserve per-element literal precision.

```ts
declare function withValue<T>(value: T, callback: (read: () => T) => void): void;
declare function forward<T>(value: T, callback: (value: T) => void): void;

const tuple = [1, 2] as const;

forward(tuple, payload => {
    withValue(payload, read => {
        const current = read();
        current[0] satisfies 1;
        current[1] satisfies 2;
    });
});
```

### nested callback forwarding widens mutable tuple element reads

Forwarding mutable tuple-like inputs through the same path should widen element reads to mutable primitives.

```ts
declare function withValue<T>(value: T, callback: (read: () => T) => void): void;
declare function forward<T>(value: T, callback: (value: T) => void): void;

let tuple = [1, 2];

forward(tuple, payload => {
    withValue(payload, read => {
        const current = read();
        current[0] satisfies 1;
    });
});
```

- contains: not assignable

### returned generic callback preserves const tuple precision

Generic callbacks returned from closures should still preserve const tuple precision across that boundary.

```ts
declare function bind<T>(value: T): <U>(callback: (value: T) => U) => U;

const run = bind([1, 2] as const);
const head = run(tuple => tuple[0]);

head satisfies 1;
```

### returned generic callback widens mutable tuple precision

The returned-callback path should not preserve literal tuple precision for mutable tuple inputs.

```ts
declare function bind<T>(value: T): <U>(callback: (value: T) => U) => U;

const run = bind([1, 2]);
const head = run(tuple => tuple[0]);

head satisfies 1;
```

- contains: not assignable

### returned generic callback precision survives renamed re exports

Const tuple precision through returned callbacks should survive renamed re-export routing boundaries.

```ts:api.ts
export declare function bind<T>(value: T): <U>(callback: (value: T) => U) => U;
```

```ts:index.ts
export { bind as makeBinder } from "./api";
```

```ts:main.ts
import { makeBinder } from "./index";

const run = makeBinder([1, 2] as const);
const head = run(tuple => tuple[0]);

head satisfies 1;
```

## freshness and excess property checks

### fresh discriminant object literal rejects extra fields through generic wrappers

Fresh discriminant literals passed through generic wrappers should still trigger excess-property rejection.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function consume<T>(value: T, callback: (value: T) => void): void;
declare function accept(input: Ready | Idle): void;

consume({ kind: "ready", payload: "ok", extra: true }, value => {
    accept(value);
});
```

- contains: not assignable

### stale discriminant object values allow extra fields through variable indirection

The same shape should become assignable after variable indirection makes the literal stale.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function accept(input: Ready | Idle): void;

const value = { kind: "ready" as const, payload: "ok", extra: true };
accept(value);
```

### direct fresh discriminant literals reject extra fields

Direct fresh discriminant literals should reject excess properties at the assignment site.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function accept(input: Ready | Idle): void;

accept({ kind: "ready", payload: "ok", extra: true });
```

- contains: excess property

### generic wrapping keeps stale discriminant values assignable

Wrapping and unwrapping stale discriminant values should keep them assignable without re-freshening.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function wrap<T>(value: T): { value: T };
declare function unwrap<T>(value: { value: T }): T;
declare function accept(input: Ready | Idle): void;

const boxed = wrap({ kind: "ready" as const, payload: "ok", extra: true });
const value = unwrap(boxed);

accept(value);
```

## distributive conditional and indexed access interactions

### indexed access over optional union members includes undefined

Indexed access over optional members in a union should include `undefined` in the resulting value type.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const maybe: Value = undefined;
maybe satisfies number | string | undefined;
```

### indexed access over optional union members rejects assignment to missing required value

Those optional-indexed projections should reject assignment into required-only target projections.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const maybe: Value = undefined;
maybe satisfies number | string;
```

- contains: not assignable

### indexed access over optional union members rejects unrelated values

Optional-union indexed projections should also reject values outside the projected member union.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const bad: Value = true;
```

- contains: not assignable

### distributive conditional argument extraction preserves union members

Distributive conditional extraction of function arguments should preserve each union member contribution.

```ts
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const first: Input = "ok";
const second: Input = 1;
```

### distributive conditional argument extraction rejects unrelated members

That extracted argument union should reject unrelated assignments not present in any branch.

```ts
type Argument<T> = T extends (value: infer A) => unknown ? A : never;

type Input = Argument<((value: string) => void) | ((value: number) => void)>;

const bad: Input = false;
```

- contains: not assignable

### recursive template literal parameter extraction keeps all path params

Recursive template-literal parameter extraction should keep every parameter name discovered along the path.

```ts
type Params<T extends string> =
    T extends `${string}:${infer Param}/${infer Rest}`
        ? Param | Params<Rest>
        : T extends `${string}:${infer Param}`
            ? Param
            : never;

type RouteParams = Params<"/users/:userId/posts/:postId">;

declare const key: RouteParams;
key satisfies "userId" | "postId";
```

### recursive template literal parameter extraction rejects unrelated params

The extracted parameter-name union should reject names that never appear in the template pattern.

```ts
type Params<T extends string> =
    T extends `${string}:${infer Param}/${infer Rest}`
        ? Param | Params<Rest>
        : T extends `${string}:${infer Param}`
            ? Param
            : never;

type RouteParams = Params<"/users/:userId/posts/:postId">;

declare const key: RouteParams;
key satisfies "userId" | "postId" | "commentId";
```

- contains: not assignable

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```
