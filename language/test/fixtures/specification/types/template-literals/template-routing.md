# Template Literal Routing

## empty captures

### literal boundaries can capture empty spans

> When both delimiters are present, template capture segments should allow empty-string matches between literals.

```ds
declare function strip<T extends string>(value: `/${T}/`): T;

const segment = strip("//");
segment satisfies "";
```

## higher-order composition

### callback routing preserves repeated-span unions

> Repeated template spans inferred through callback parameters should keep their union constraints after routing.

```ds
declare function route<T extends string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const input = true ? "id:users" : "id:posts";
const output = route(input, segment => `${segment}:${segment}`);

output satisfies "users:users" | "posts:posts";
```

### nested combinators preserve inferred segment literals

> Composed generic parser combinators should preserve literal segment inference across nested calls.

```ds
declare function parse<T extends string>(value: `id:${T}`): T;
declare function map<T, U>(value: T, callback: (value: T) => U): U;

const value = map(parse("id:orders"), segment => `#${segment}` as const);
value satisfies "#orders";
```

## module routing

### namespace export chains preserve parser constraints

> Template-parser constraints should remain intact when helpers are routed through namespace export chains.

```ds:helper.ds
export declare function parse<T extends string>(value: `id:${T}`): T;
```

```ds:barrel.ds
export * from "./helper";
```

```ds:index.ds
export * as api from "./barrel";
```

```ds:main.ds
import { api } from "./index";

const value = api.parse("id:users");
value satisfies "users";
```

### namespace export chains reject non matching parser inputs

> The same namespace routing should still reject inputs that do not match required template boundaries.

```ds:helper.ds
export declare function parse<T extends string>(value: `id:${T}`): T;
```

```ds:barrel.ds
export * from "./helper";
```

```ds:index.ds
export * as api from "./barrel";
```

```ds:main.ds
import { api } from "./index";

api.parse("users");
```

- contains: not assignable
