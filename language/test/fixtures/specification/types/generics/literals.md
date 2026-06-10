# Literal Inference

Literal types survive inference where the context keeps them.

## local literals

### generic inference preserves const literal precision

Const literal arguments preserve literal precision during generic inference.

```ds
declare function id<T>(value: T): T;

const value = "ready";
const result = id(value);

result satisfies "ready";
```

### generic inference widens let literal sources

Mutable literal sources infer widened primitive types.

```ds
declare function id<T>(value: T): T;

let value = "ready";
let result = id(value);

result satisfies string;
```

### generic inference does not restore literals from widened let sources

Generic inference does not recover lost literal freshness from widened sources.

```ds
declare function id<T>(value: T): T;

let value = "ready";
let result = id(value);

result satisfies "ready";
```

- contains: not assignable

## imports

### imported generic inference preserves const literal precision

Imported generic calls keep const literal precision at the call site.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import { id } from "./helper.ds";

const value = "ready";
const result = id(value);

result satisfies "ready";
```

### imported generic inference widens let literal sources

Imported generic calls infer widened primitive types for mutable sources.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import { id } from "./helper.ds";

let value = "ready";
let result = id(value);

result satisfies string;
```

### imported generic inference from let does not restore literal precision

Imported generic calls do not recover literal precision from widened mutable sources.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import { id } from "./helper.ds";

let value = "ready";
let result = id(value);

result satisfies "ready";
```

- contains: not assignable

### renamed re-export generic inference preserves const literal precision

Renamed re-exports preserve const literal precision at imported call sites.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:index.ds
export { id as identity } from "./helper.ds";
```

```ds:main.ds
import { identity } from "./index.ds";

const value = "ready";
const result = identity(value);

result satisfies "ready";
```

### export-star generic inference keeps let widening

Export-star forwarding keeps mutable-source widening.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:index.ds
export * from "./helper.ds";
```

```ds:main.ds
import { id } from "./index.ds";

let value = "ready";
const result = id(value);

result satisfies string;
```

### namespace import generic inference preserves const ternary literal unions

Namespace imports preserve const ternary union precision at generic call sites.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import * as api from "./helper.ds";

const value = true ? "api" : "admin";
const result = api.id(value);

result satisfies "api" | "admin";
```

### namespace import generic inference keeps let ternary widening

Namespace imports keep mutable ternary widening at generic call sites.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import * as api from "./helper.ds";

let value = true ? "api" : "admin";
const result = api.id(value);

result satisfies string;
```

### generic inference preserves const ternary literal unions

Const ternary inputs keep literal unions through generic inference.

```ds
declare function id<T>(value: T): T;

const value = true ? "api" : "admin";
const result = id(value);

result satisfies "api" | "admin";
```

### generic inference widens let ternary literal unions

Mutable ternary inputs widen through generic inference.

```ds
declare function id<T>(value: T): T;

let value = true ? "api" : "admin";
let result = id(value);

result satisfies string;
```

### generic inference from let ternary unions does not keep literal unions

Generic inference on widened ternary values does not keep literal unions.

```ds
declare function id<T>(value: T): T;

let value = true ? "api" : "admin";
let result = id(value);

result satisfies "api" | "admin";
```

- contains: not assignable

## constraints

### constrained generic inference keeps const literal precision

Const literals satisfy constrained generic parameters with literal precision.

```ds
declare function choose<T: "dev" | "prod">(value: T): T;

const mode = "dev";
const result = choose(mode);

result satisfies "dev";
```

### constrained generic inference rejects widened let literals

Widened mutable literals do not satisfy constrained literal generic parameters.

```ds
declare function choose<T: "dev" | "prod">(value: T): T;

let mode = "dev";
choose(mode);
```

- contains: not assignable

### constrained generic inference keeps const ternary literal unions

Const ternary unions remain precise when inferring constrained generic parameters.

```ds
declare function choose<T: "dev" | "prod">(value: T): T;

const mode = true ? "dev" : "prod";
const result = choose(mode);

result satisfies "dev" | "prod";
```

### constrained generic inference rejects widened let ternary literals

Mutable ternary literals widen and fail constrained literal generic inference.

```ds
declare function choose<T: "dev" | "prod">(value: T): T;

let mode = true ? "dev" : "prod";
choose(mode);
```

- contains: not assignable

## objects and templates

### constrained generic inference keeps const object discriminants

Const object literals preserve discriminants through constrained generic inference.

```ds
declare function select<T: { kind: "a" | "b" }>(value: T): T;

const value = { kind: "a" as const, payload: 1 };
const result = select(value);

result.kind satisfies "a";
```

### constrained generic inference rejects widened object discriminants

Widened object discriminants do not satisfy constrained literal generic inference.

```ds
declare function select<T: { kind: "a" | "b" }>(value: T): T;

let value = { kind: "a", payload: 1 };
select(value);
```

- contains: not assignable

### constrained template inference keeps const span literals

Template span inference keeps const span literals under constrained generics.

```ds
declare function parse<T: "users" | "posts">(value: `id:${T}`): T;

const value = "id:users";
const result = parse(value);

result satisfies "users";
```

### constrained template inference rejects widened let strings

Widened mutable strings do not satisfy constrained template span generics.

```ds
declare function parse<T: "users" | "posts">(value: `id:${T}`): T;

let value = "id:users";
parse(value);
```

- contains: not assignable

## source bindings

### constrained calls do not change later let bindings

A constrained generic call keeps its own literal result, but a later `let` from the same source still widens.

```ds
declare function as_lit<T: string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### later let bindings do not regain literals

The widened binding is not assignable to the original literal type.

```ds
declare function as_lit<T: string>(value: T): T;

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

widened satisfies "users";
```

- contains: not assignable

### imported constrained calls keep local widening

Cross-module calls use the imported signature, and the later `let` is still inferred locally.

```ds:lib.ds
export function as_lit<T: string>(value: T): T {
    return value;
}
```

```ds:main.ds
import { as_lit } from "./lib.ds";

const seed = "users";
const kept = as_lit(seed);
let widened = seed;

kept satisfies "users";
widened satisfies string;
```

### parameter defaults are widened sources

Default parameter values infer the parameter type, not a fresh literal.

```ds
declare function as_lit<T: string>(value: T): T;

function read(mode = "users") {
    const kept = as_lit(mode);
    let widened = mode;

    kept satisfies string;
    widened satisfies string;
}
```
