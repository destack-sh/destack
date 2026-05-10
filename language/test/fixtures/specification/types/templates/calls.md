# Template Calls

## inference

### template literal infers from call arguments

Template literal parameters can drive generic inference.

```ds
declare function take<T>(value: `${T}`): T;

let value = take("hello");
value satisfies "hello";
```

### template literal infers from template literal arguments

Template literal arguments can infer span types.

```ds
declare function take<T: string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
result satisfies "a" | "b";
```

### template literal infers from template literal arguments rejects narrowed result

Inferred spans preserve union members.

```ds
declare function take<T: string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers constrained spans from call arguments

Call inference respects constrained template spans.

```ds
declare function take<T: "a" | "b">(value: `prefix-${T}`): T;

let ok = take("prefix-a");
ok satisfies "a";
```

### template literal rejects call arguments outside constraint

Constrained spans reject values outside the union.

```ds
declare function take<T: "a" | "b">(value: `prefix-${T}`): T;

let bad = take("prefix-c");
```

- contains: not assignable

### template literal infers from template literal parameters

Template literal arguments flow into generic spans.

```ds
declare function takeAny<T: string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
result satisfies `prefix-${"a"}`;
```

### template literal infers from template literal parameters rejects narrowed result

Inference preserves the full template literal shape.

```ds
declare function takeAny<T: string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers empty span with literal boundary

Literal boundaries allow empty captures for call inference.

```ds
declare function take<T>(value: `a${T}`): T;

let ok = take("a");
ok satisfies "";
```

## literal precision

### template literal argument inference keeps const literal precision

Const literals preserve the captured span for template argument inference.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

const value = "id:users";
const result = parse(value);

result satisfies "users";
```

## imports

### imported template parser keeps const literal precision

Imported template parser calls keep const literal precision at the call site.

```ds:helper.ds
export function parse<T: string>(value: `id:${T}`): T {
    return "users" as T;
}
```

```ds:main.ds
import { parse } from "./helper.ds";

const value = "id:users";
const result = parse(value);

result satisfies "users";
```

### imported template parser rejects widened let scalar inputs

Imported template parser calls still reject widened mutable scalar inputs.

```ds:helper.ds
export function parse<T: string>(value: `id:${T}`): T {
    return "users" as T;
}
```

```ds:main.ds
import { parse } from "./helper.ds";

let value = "id:users";
parse(value);
```

- contains: not assignable to type `id:${string}`

### imported generic `${T}` parser keeps const scalar precision

Imported generic `${T}` parser calls keep const scalar precision.

```ds:helper.ds
export function identitySpan<T: string>(value: `${T}`): T {
    return "users" as T;
}
```

```ds:main.ds
import { identitySpan } from "./helper.ds";

const value = "users";
const result = identitySpan(value);

result satisfies "users";
```

### template literal argument inference keeps contextual template union precision

Contextual template unions preserve span unions for template argument inference.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

const value: `id:${"users" | "posts"}` = true ? "id:users" : "id:posts";
const result = parse(value);

result satisfies "users" | "posts";
```

### template literal argument inference rejects widened let scalar inputs

Widened `let` string inputs do not satisfy narrow template argument shapes.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

let value = "id:users";
parse(value);
```

- contains: not assignable to type `id:${string}`

### template literal argument inference accepts widened let scalar inputs for `${T}`

Widened `let` scalar inputs infer `T = string` for generic `${T}` positions.

```ds
declare function identitySpan<T: string>(value: `${T}`): T;

let value = "users";
const result = identitySpan(value);

result satisfies string;
```

### template literal argument inference keeps const scalar inputs for `${T}`

Const scalar inputs keep literal precision through `${T}` argument inference.

```ds
declare function identitySpan<T: string>(value: `${T}`): T;

const value = "users";
const result = identitySpan(value);

result satisfies "users";
```

### template literal argument inference preserves const ternary unions for `${T}`

Const ternary unions preserve span unions in generic `${T}` argument inference.

```ds
declare function identitySpan<T: string>(value: `${T}`): T;

const value = true ? "users" : "posts";
const result = identitySpan(value);

result satisfies "users" | "posts";
```

### renamed re-export template parser keeps const literal precision

Renamed re-exports preserve template argument inference precision.

```ds:helper.ds
export function parse<T: string>(value: `id:${T}`): T {
    return "users" as T;
}
```

```ds:index.ds
export { parse as parseId } from "./helper.ds";
```

```ds:main.ds
import { parseId } from "./index.ds";

const value = "id:users";
const result = parseId(value);

result satisfies "users";
```

### export-star template parser rejects widened let scalar inputs

Export-star forwarding preserves narrow template rejection for widened mutable inputs.

```ds:helper.ds
export function parse<T: string>(value: `id:${T}`): T {
    return "users" as T;
}
```

```ds:index.ds
export * from "./helper.ds";
```

```ds:main.ds
import { parse } from "./index.ds";

let value = "id:users";
parse(value);
```

- contains: not assignable to type `id:${string}`

### namespace import generic `${T}` parser keeps const scalar precision

Namespace imports preserve generic `${T}` inference for const literals.

```ds:helper.ds
export function identitySpan<T: string>(value: `${T}`): T {
    return "users" as T;
}
```

```ds:main.ds
import * as api from "./helper.ds";

const value = "users";
const result = api.identitySpan(value);

result satisfies "users";
```

### template literal parser infers union spans from const ternary arguments

Const ternary arguments preserve union span precision for template literal inference.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

const value = true ? "id:users" : "id:posts";
const result = parse(value);

result satisfies "users" | "posts";
```

### template literal parser rejects mutable ternary arguments for narrow templates

Mutable ternary arguments widen and no longer satisfy narrow template literal shapes.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

let value = true ? "id:users" : "id:posts";
parse(value);
```

- contains: not assignable to type `id:${string}`

### renamed re-export generic `${T}` parser keeps const precision

Renamed re-exports preserve generic `${T}` const literal precision.

```ds:helper.ds
export function identitySpan<T: string>(value: `${T}`): T {
    return "users" as T;
}
```

```ds:index.ds
export { identitySpan as span } from "./helper.ds";
```

```ds:main.ds
import { span } from "./index.ds";

const value = "users";
const result = span(value);

result satisfies "users";
```

### export star generic `${T}` parser widens mutable scalar inputs

Export-star forwarding preserves mutable scalar widening for generic `${T}` inference.

```ds:helper.ds
export function identitySpan<T: string>(value: `${T}`): T {
    return "users" as T;
}
```

```ds:index.ds
export * from "./helper.ds";
```

```ds:main.ds
import { identitySpan } from "./index.ds";

let value = "users";
const result = identitySpan(value);

result satisfies string;
```

### template inference keeps const object member literal precision

Const object members preserve literal precision through `${T}` inference.

```ds
declare function identitySpan<T: string>(value: `${T}`): T;

const config = { id: "users" } as const;
const result = identitySpan(config.id);

result satisfies "users";
```

### template inference widens mutable object member literals

Mutable object members widen before `${T}` argument inference.

```ds
declare function identitySpan<T: string>(value: `${T}`): T;

let config = { id: "users" };
const result = identitySpan(config.id);

result satisfies string;
```

### template inference rejects widened mutable object members for narrow templates

Widened mutable object members do not satisfy narrow template literal argument shapes.

```ds
declare function parse<T: string>(value: `id:${T}`): T;

let config = { id: "id:users" };
parse(config.id);
```

- contains: not assignable to type `id:${string}`
