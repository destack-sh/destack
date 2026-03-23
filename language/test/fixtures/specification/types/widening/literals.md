# Literal Widening

Tests for TypeScript-style literal widening, freshness, and const contexts.

## Local bindings

### let scalar literals widen

`let` bindings without a constraining type widen scalar literals.

```ts
let value = 1;

value satisfies number;
```

### let scalar literals are not kept as literals

Widened `let` bindings are not assignable to the original literal type.

```ts
let value = 1;

value satisfies 1;
```

- contains: not assignable

### const scalar literals keep literal types

`const` bindings keep scalar literal types.

```ts
const value = 1;

value satisfies 1;
```

### const object members widen without const context

`const` does not implicitly freeze object members without a const context.

```ts
const config = { version: 1 };

config.version satisfies number;
```

### const object members are not kept as literals by default

Object members still widen without a const context.

```ts
const config = { version: 1 };

config.version satisfies 1;
```

- contains: not assignable

### const assertions suppress widening for object members

Const assertions suppress widening and preserve literal members.

```ts
const config = { version: 1 } as const;

config.version satisfies 1;
```

### const assertions preserve nested object members

Const assertions are deep and preserve nested literal members.

```ts
const config = { nested: { mode: "dev" } } as const;

config.nested.mode satisfies "dev";
```

### const assertions suppress widening for tuples

Const assertions preserve tuple literal members.

```ts
const pair = [1, 2] as const;

pair[0] satisfies 1;
pair[1] satisfies 2;
```

### const assertions preserve object literal array members

Const assertions preserve literal members in array elements.

```ts
const values = [{ kind: "a" }, { kind: "b" }] as const;

values[0].kind satisfies "a";
values[1].kind satisfies "b";
```

### const assertions preserve nested array members

Const assertions are deep for nested array literals.

```ts
const grid = [[1, 2]] as const;

grid[0][0] satisfies 1;
grid[0][1] satisfies 2;
```

### const assertions reject non literal conditional expressions

Const assertions are only valid on direct literal forms, not whole conditional expressions.

```ts
const value = (true ? 1 : 2) as const;
```

- const assertions

### const arrays widen without const assertions

Array literals widen their element types without const assertions.

```ts
const pair = [1, 2];

pair[0] satisfies number;
```

### const arrays do not keep literal elements by default

Widened array elements are not assignable to the original literal element types.

```ts
const pair = [1, 2];

pair[0] satisfies 1;
```

- contains: not assignable

### annotations prevent widening but do not keep literal types

Contextual types prevent widening to primitives but do not keep literal types.

```ts
let value: 1 | 2 = 1;

value satisfies 1 | 2;
value satisfies 1;
```

- contains: not assignable

## Cross module surfaces

### exported const literals keep literal types across modules

Exported `const` literals keep their literal types across module boundaries.

```ts:values.ts
export const version = 1;
```

```ts:main.ts
import { version } from "./values";

version satisfies 1;
```

### exported let literals widen across modules

Exported `let` literals widen to their primitive types across module boundaries.

```ts:values.ts
export let counter = 1;
```

```ts:main.ts
import { counter } from "./values";

counter satisfies 1;
```

- contains: not assignable

### exported let literals still satisfy primitive types

Exported widened `let` literals still satisfy their primitive types.

```ts:values.ts
export let counter = 1;
```

```ts:main.ts
import { counter } from "./values";

counter satisfies number;
```

### exported const assertions preserve literal members across modules

Const assertions on exports preserve literal members across module boundaries.

```ts:values.ts
export const config = { version: 1 } as const;
```

```ts:main.ts
import { config } from "./values";

config.version satisfies 1;
```

### exported const objects still widen members without const assertions

Exported `const` object members widen without const assertions.

```ts:values.ts
export const config = { version: 1 };
```

```ts:main.ts
import { config } from "./values";

config.version satisfies 1;
```

- contains: not assignable

### renamed re-export const literals keep literal types across modules

Renamed re-exports should preserve exported const literal precision.

```ts:values.ts
export const version = 1;
```

```ts:index.ts
export { version as publicVersion } from "./values";
```

```ts:main.ts
import { publicVersion } from "./index";

publicVersion satisfies 1;
```

### export-star forwarded let literals still widen across modules

Export-star forwarding should preserve widened `let` literal behavior.

```ts:values.ts
export let counter = 1;
```

```ts:index.ts
export * from "./values";
```

```ts:main.ts
import { counter } from "./index";

counter satisfies number;
```

### namespace imports preserve const assertion literal members

Namespace imports should preserve const assertion literal member precision.

```ts:values.ts
export const config = { version: 1 } as const;
```

```ts:main.ts
import * as values from "./values";

values.config.version satisfies 1;
```

## Assignment behavior

### assigning const scalar literals into let bindings widens

Fresh const scalar literals widen when assigned into mutable bindings.

```ds
const seed = 1;
let value = seed;

value satisfies number;
```

### assigning const scalar literals into let bindings does not keep literal

Mutable bindings do not preserve the original scalar literal.

```ds
const seed = 1;
let value = seed;

value satisfies 1;
```

- contains: not assignable

### assigning widened let scalars into const bindings keeps widened type

Const bindings do not re-narrow already widened sources.

```ds
let seed = "ready";
const value = seed;

value satisfies string;
```

## inference interactions

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

Constrained generic inference regularizes literal arguments instead of widening at binding commitment.

```ds
declare function as_lit<T extends string>(value: T): T;

let value = as_lit("users");
value satisfies "users";
```

### constrained generic regularization does not consume source freshness

Constrained generic regularization should not mutate unrelated later commitment points.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained generic regularization still allows later widening

Later let commitments from the same source should still widen after constrained generic calls.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained generic regularization through alias chains keeps source widenability

Alias chains should not consume source freshness for later commitment points.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const alias = seed;
const kept = as_lit(alias);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained generic regularization through alias chains still widens later lets

Alias chain calls should still allow widening at later let commitments.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const alias = seed;
const kept = as_lit(alias);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained generic regularization on tuple elements keeps source widenability

Tuple element constrained calls should not consume source freshness for later let commitments.

```ds
declare function as_lit<T extends string>(value: T): T;

const pair = ["users", "posts"] as const;
const kept = as_lit(pair[0]);
let widened = pair[0];

kept satisfies "users";
widened satisfies "users";
```

### constrained generic regularization on tuple elements rejects unrelated literals

Tuple element constrained calls should still reject unrelated literals.

```ds
declare function as_lit<T extends string>(value: T): T;

const pair = ["users", "posts"] as const;
const kept = as_lit(pair[0]);
let widened = pair[0];

widened satisfies "posts";
```

- contains: not assignable

### constrained template argument inference keeps source widenability

Template constrained calls should not consume source freshness for later let commitments.

```ds
declare function identity_span<T extends string>(value: `${T}`): T;

const seed = "users";
const kept = identity_span(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### constrained template argument inference still widens later lets

Template constrained calls should still allow widening at later let commitments.

```ds
declare function identity_span<T extends string>(value: `${T}`): T;

const seed = "users";
const kept = identity_span(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained overload paths keep source widenability

Constrained overload resolution should not consume source freshness for later let commitments.

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

Constrained overload resolution should still allow widening at later let commitments.

```ds
declare function overload_lit<T extends string>(value: T): T;
declare function overload_lit(value: string): string;

const seed = "users";
const kept = overload_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### repeated constrained calls keep source widenability

Multiple constrained calls from one source should not consume later let widening.

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

Multiple constrained calls should still allow widening at later let commitments.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const first = as_lit(seed);
const second = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### cross module constrained generic calls keep source widenability

Constrained generic calls across module boundaries should not consume local source freshness.

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

Cross module constrained generic calls should still allow widening at later let commitments.

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

### constrained generic member paths keep source widenability

Constrained calls through object member paths should not consume source freshness for later let commitments.

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

Constrained member-path calls should still allow widening at later let commitments.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users";
const holder = { seed } as const;
const kept = as_lit(holder.seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### constrained generic calls from parameter defaults keep widening behavior

Parameter default commitment should remain widened even after constrained generic calls.

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

Parameter defaults should not keep narrow literals after constrained generic calls.

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

Satisfies expressions should still widen later let commitments.

```ds
declare function as_lit<T extends string>(value: T): T;

const seed = "users" satisfies string;
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### satisfies boundary with constrained calls does not keep literals

Satisfies expressions should not keep narrow literals across later let commitments.

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

Const bindings preserve source precision, not original initializer freshness.

```ds
let seed = "ready";
const value = seed;

value satisfies "ready";
```

- contains: not assignable

### nested literal usage does not consume const source freshness

Using a const literal inside nested literal inference should not prevent later let commitment widening.

```ds
const seed = "ready";
const holder = { seed };
let value = seed;

value satisfies string;
```

## Defaults and returns

### parameter defaults widen literal initializers

Parameter default literals widen to primitive parameter types in function bodies.

```ts
function readMode(mode = "dev") {
    mode satisfies string;
}
```

### parameter defaults do not keep literal initializers

Parameter default literals are not preserved as literal types by default.

```ts
function readMode(mode = "dev") {
    mode satisfies "dev";
}
```

- contains: not assignable

### function return inference widens literal returns

Function return inference widens unconstrained literal return expressions.

```ts
function makeMode() {
    return "dev";
}

const mode = makeMode();
mode satisfies string;
```

### function return inference does not keep literal returns

Unconstrained return inference does not preserve literal return values for plain function declarations.

```ts
function makeMode() {
    return "dev";
}

const mode = makeMode();
mode satisfies "dev";
```

- contains: not assignable

## Nested boundaries

### nested function return inference widens literal returns

Nested function declarations should widen unconstrained literal return values.

```ts
function outer() {
    function inner() {
        return "dev";
    }

    const mode = inner();
    mode satisfies string;
}
```

### nested function return inference does not keep literal returns

Nested function declarations should not preserve literal return values by default.

```ts
function outer() {
    function inner() {
        return "dev";
    }

    const mode = inner();
    mode satisfies "dev";
}
```

- contains: not assignable

### nested const to let commitment still widens after inner usage

Using a const literal inside a nested function should not prevent later mutable widening.

```ds
const seed = "ready";

function observe() {
    seed satisfies "ready";
}

let widened = seed;
widened satisfies string;
```

### nested const to let commitment does not keep literal after inner usage

Nested reads should not keep mutable commitments pinned to the original literal.

```ds
const seed = "ready";

function observe() {
    seed satisfies "ready";
}

let widened = seed;
widened satisfies "ready";
```

- contains: not assignable
