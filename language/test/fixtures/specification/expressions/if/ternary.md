# Ternary Expressions

Ternary expression typing.

## typing

### ternary yields union of branch types

> Ternary expressions produce the union of branch types.

```ds
const value = true ? 1 : "hi";
value satisfies int | string;
```

### ternary respects contextual type

> Contextual types constrain ternary branches.

```ds
const value: int = true ? 1 : 2;
value satisfies int;
```

### ternary rejects incompatible branch

> Branches must satisfy the contextual type.

```ds
const value: int = true ? 1 : "hi";
```

- contains: not assignable

## literal precision

### let ternary literal unions widen

`let` bindings widen ternary literal unions to primitive types.

```ds
let value = true ? "api" : "admin";

value satisfies string;
```

### let ternary literal unions do not keep literal unions

Widened `let` ternary results do not keep literal unions.

```ds
let value = true ? "api" : "admin";

value satisfies "api" | "admin";
```

- contains: not assignable

## cross-module literal precision

### exported const ternary values preserve literal unions across modules

Const ternary exports preserve literal unions through module boundaries.

```ds:values.ds
export const route = true ? "api" : "admin";
```

```ds:main.ds
import { route } from "./values";

route satisfies "api" | "admin";
```

### exported let ternary values widen across modules

Let ternary exports widen across module boundaries.

```ds:values.ds
export let route = true ? "api" : "admin";
```

```ds:main.ds
import { route } from "./values";

route satisfies string;
```

### exported let ternary values do not keep literal unions across modules

Widened let ternary exports do not keep literal unions through module boundaries.

```ds:values.ds
export let route = true ? "api" : "admin";
```

```ds:main.ds
import { route } from "./values";

route satisfies "api" | "admin";
```

- contains: not assignable

### const ternary literal unions preserve literal unions

`const` bindings keep ternary literal unions.

```ds
const value = true ? "api" : "admin";

value satisfies "api" | "admin";
```

### const ternary object members widen without const context

Const ternary object bindings still widen member literals without const assertions.

```ds
const route = true ? { kind: "api" } : { kind: "admin" };

route.kind satisfies string;
```

### const ternary object members do not keep member literal unions

Const ternary object bindings do not keep member literal unions by default.

```ds
const route = true ? { kind: "api" } : { kind: "admin" };

route.kind satisfies "api" | "admin";
```

- contains: not assignable

### contextual ternary unions reject narrowed expectations

Contextual union types do not narrow to one branch value.

```ds
const value: "api" | "admin" = true ? "api" : "admin";

value satisfies "api";
```

- contains: not assignable

### let ternary object members widen without const context

Mutable ternary object bindings widen member literals.

```ds
let route = true ? { kind: "api" } : { kind: "admin" };

route.kind satisfies string;
```

### let ternary object members do not keep member literals

Mutable ternary object bindings do not keep member literal unions.

```ds
let route = true ? { kind: "api" } : { kind: "admin" };

route.kind satisfies "api" | "admin";
```

- contains: not assignable

### exported const ternary object members widen across modules

Const ternary object exports still widen member literals across module boundaries.

```ds:values.ds
export const route = true ? { kind: "api" } : { kind: "admin" };
```

```ds:main.ds
import { route } from "./values";

route.kind satisfies string;
```

### exported const ternary object members do not keep literal unions across modules

Const ternary object exports do not keep member literal unions without const assertions.

```ds:values.ds
export const route = true ? { kind: "api" } : { kind: "admin" };
```

```ds:main.ds
import { route } from "./values";

route.kind satisfies "api" | "admin";
```

- contains: not assignable

### branch const assertions preserve ternary object member literal unions

Const assertions on both ternary branches preserve object member literal unions.

```ds
const route = true
    ? ({ kind: "api" } as const)
    : ({ kind: "admin" } as const);

route.kind satisfies "api" | "admin";
```

### branch const assertions preserve ternary object member literal unions across modules

Branch-level const assertions preserve object member literal unions across module boundaries.

```ds:values.ds
export const route = true
    ? ({ kind: "api" } as const)
    : ({ kind: "admin" } as const);
```

```ds:main.ds
import { route } from "./values";

route.kind satisfies "api" | "admin";
```

### explicit export annotations anchor ternary literal unions across modules

Explicit export annotations preserve ternary literal union contracts across module boundaries.

```ds:values.ds
export const route: "api" | "admin" = true ? "api" : "admin";
```

```ds:main.ds
import { route } from "./values";

route satisfies "api" | "admin";
```

## generic inference

### const ternary unions preserve literal precision in generic inference

Const ternary literal unions preserve branch precision in unconstrained generic calls.

```ds
declare function identity<T>(value: T): T;

const value = true ? "api" : "admin";
const result = identity(value);

result satisfies "api" | "admin";
```

### let ternary unions widen before generic inference

Mutable ternary literal unions widen before unconstrained generic calls.

```ds
declare function identity<T>(value: T): T;

let value = true ? "api" : "admin";
const result = identity(value);

result satisfies string;
```

### let ternary unions do not keep literal precision in generic inference

Widened mutable ternary unions do not keep branch literal precision in generic calls.

```ds
declare function identity<T>(value: T): T;

let value = true ? "api" : "admin";
const result = identity(value);

result satisfies "api" | "admin";
```

- contains: not assignable

### const ternary templates preserve span unions in generic template inference

Const ternary template unions preserve span unions in template-argument inference.

```ds
declare function parse<T extends string>(value: `id:${T}`): T;

const route = true ? "id:users" : "id:posts";
const span = parse(route);

span satisfies "users" | "posts";
```

### let ternary templates are rejected by narrow template argument shapes

Mutable ternary templates widen before template-argument inference and fail narrow template matches.

```ds
declare function parse<T extends string>(value: `id:${T}`): T;

let route = true ? "id:users" : "id:posts";
parse(route);
```

- contains: not assignable to type `id:${string}`
