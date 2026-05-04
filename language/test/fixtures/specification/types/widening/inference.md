# Inference

## callbacks

### nested callback forwarding preserves const tuple element precision

Forwarding const tuples through nested higher-order callbacks preserves per-element literal precision.

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

Forwarding mutable tuple-like inputs through the same path widens element reads to mutable primitives.

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

Generic callbacks returned from closures preserve const tuple precision across that boundary.

```ts
declare function bind<T>(value: T): <U>(callback: (value: T) => U) => U;

const run = bind([1, 2] as const);
const head = run(tuple => tuple[0]);

head satisfies 1;
```

### returned generic callback widens mutable tuple precision

Returned callbacks do not preserve literal tuple precision for mutable tuple inputs.

```ts
declare function bind<T>(value: T): <U>(callback: (value: T) => U) => U;

const run = bind([1, 2]);
const head = run(tuple => tuple[0]);

head satisfies 1;
```

- contains: not assignable

### returned generic callback precision survives renamed exports

Const tuple precision through returned callbacks survives renamed re-exports.

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

## object freshness

### fresh discriminant object literal rejects extra fields through generic wrappers

Fresh discriminant literals passed through generic wrappers trigger excess-property rejection.

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

The same shape becomes assignable after variable indirection makes the literal stale.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function accept(input: Ready | Idle): void;

const value = { kind: "ready" as const, payload: "ok", extra: true };
accept(value);
```

### direct fresh discriminant literals reject extra fields

Direct fresh discriminant literals reject excess properties at the assignment site.

```ts
type Ready = { kind: "ready", payload: string };
type Idle = { kind: "idle" };

declare function accept(input: Ready | Idle): void;

accept({ kind: "ready", payload: "ok", extra: true });
```

- contains: excess property

### generic wrapping keeps stale discriminant values assignable

Wrapping and unwrapping stale discriminant values keep them assignable without re-freshening.

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

## literal arguments

### const literal arguments keep literal precision through generic inference

Const literal arguments preserve literal precision through unconstrained generic calls.

```ds
declare function identity<T>(value: T): T;

const value = identity("users");
value satisfies "users";
```

### let literal arguments widen before unconstrained generic inference

Mutable literal arguments widen before unconstrained generic calls.

```ds
declare function identity<T>(value: T): T;

let value = "users";
const result = identity(value);
result satisfies string;
```

### let literal arguments do not keep literal precision in generic inference

Widened mutable literal arguments do not keep literal precision through unconstrained generic calls.

```ds
declare function identity<T>(value: T): T;

let value = "users";
const result = identity(value);
result satisfies "users";
```

- contains: not assignable

### constrained generic arguments keep literal precision

A constrained generic call can infer a literal type from an argument that would otherwise widen in a mutable binding.

```ds
declare function as_lit<T extends string>(value: T): T;

let value = as_lit("users");
value satisfies "users";
```

### constrained generic calls preserve later widening

Using a const literal in a constrained call does not change a later mutable binding from the same source.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained generic calls still widen later lets

A later `let` binding from the same const source still widens.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### alias chains preserve later widening

Alias chains keep later mutable bindings widened.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const alias = seed;
const kept = as_lit(alias);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### alias chains still widen later lets

Alias chain calls still allow later `let` bindings to widen.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const alias = seed;
const kept = as_lit(alias);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### tuple element calls preserve later bindings

Constrained calls from tuple elements do not change later bindings from the same element.

```ds
declare function as_lit<T extends string>(value: T): T;

const pair = ["users", "posts"] as const;
const kept = as_lit(pair[0]);
let widened = pair[0];

kept satisfies "users";
widened satisfies "users";
```

### tuple element calls reject unrelated literals

Tuple element constrained calls still reject unrelated literals.

```ds
declare function as_lit<T extends string>(value: T): T;

const pair = ["users", "posts"] as const;
const kept = as_lit(pair[0]);
let widened = pair[0];

widened satisfies "posts";
```

- contains: not assignable

### constrained template inference preserves later widening

Template constrained calls do not change later `let` bindings from the same source.

```ds
declare function identity_span<T extends string>(value: `${T}`): T;

const seed = "users";
const kept = identity_span(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained template argument inference still widens later lets

Template constrained calls still allow later `let` bindings to widen.

```ds
declare function identity_span<T extends string>(value: `${T}`): T;

const seed = "users";
const kept = identity_span(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained overloads preserve later widening

Constrained overload resolution does not change later `let` bindings from the same source.

```ds
declare function overload_lit<T extends string>(value: T): T;
declare function overload_lit(value: string): string;

const seed = "users";
const kept = overload_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained overload paths still widen later lets

Constrained overload resolution still allows later `let` bindings to widen.

```ds
declare function overload_lit<T extends string>(value: T): T;
declare function overload_lit(value: string): string;

const seed = "users";
const kept = overload_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### repeated constrained calls preserve later widening

Multiple constrained calls from one source do not change later `let` widening.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const first = as_lit(seed);
const second = as_lit(seed);
let widened = seed;

first satisfies "users";
second satisfies "users";
widened satisfies string;
```

### repeated constrained calls still widen later lets

Multiple constrained calls still allow later `let` bindings to widen.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const first = as_lit(seed);
const second = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### cross module constrained calls preserve later widening

Constrained generic calls across module boundaries do not change later local widening.

```ds:lib.ds
export function as_lit<T extends string>(value: T): T {
    return value;
}
```

```ds:main.ds
import { as_lit } from "./lib";

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### cross module constrained generic calls still widen later lets

Cross-module constrained generic calls still allow later `let` bindings to widen.

```ds:lib.ds
export function as_lit<T extends string>(value: T): T {
    return value;
}
```

```ds:main.ds
import { as_lit } from "./lib";

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained member paths preserve later widening

Constrained calls through object member paths do not change later `let` bindings from the same source.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const holder = { seed } as const;
const kept = as_lit(holder.seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained generic member paths still widen later lets

Constrained member-path calls still allow later `let` bindings to widen.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const holder = { seed } as const;
const kept = as_lit(holder.seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained generic calls from parameter defaults preserve widening

Parameter defaults remain widened even after constrained generic calls.

```ds
declare function as_lit<T extends string>(value: T): T;

function read(mode = "users") {
    const kept = as_lit(mode);
    let widened = mode;

    kept satisfies string;
    widened satisfies string;
}
```

### constrained generic calls from parameter defaults do not keep literals

Parameter defaults do not keep narrow literals after constrained generic calls.

```ds
declare function as_lit<T extends string>(value: T): T;

function read(mode = "users") {
    const kept = as_lit(mode);
    let widened = mode;

    widened satisfies "users";
}
```

- contains: not assignable

### satisfies boundary with constrained calls keeps later let widening

Satisfies expressions still allow later `let` bindings to widen.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users" satisfies string;
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### satisfies boundary with constrained calls does not keep literals

Satisfies expressions do not keep narrow literals across later `let` bindings.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users" satisfies string;
const kept = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained template inference keeps parsed bigint literals

Template literal constrained inference preserves parsed bigint literal precision.

```ds
declare function parse_big<T extends bigint>(value: `${T}`): T;

let value = parse_big("-1");
value satisfies -1n;
```

### const asserted tuples are readonly at element positions

Const asserted tuple elements are readonly and reject writes.

```ds
const pair = [1, 2] as const;
pair[0] = 3;
```

- contains: readonly

### assigning widened let scalars into const bindings does not restore literal

Const bindings preserve source precision, not the original initializer literal.

```ds
let seed = "ready";
const value = seed;

value satisfies "ready";
```

- contains: not assignable

### nested literal usage preserves later widening

Using a const literal inside nested literal inference does not prevent later `let` widening.

```ds
const seed = "ready";
const holder = { seed };
let value = seed;

value satisfies string;
```
