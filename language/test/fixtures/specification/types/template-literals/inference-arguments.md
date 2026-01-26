# Template Literal Inference (Arguments)

## inference

### template literal infers from call arguments

> Generic inference can flow through template literal parameters.

```ds
declare function take<T>(value: `${T}`): T;

let value = take("hello");
value satisfies "hello";
```

### template literal infers from template literal arguments

> Template literal arguments can infer span types.

```ds
declare function take<T extends string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
result satisfies "a" | "b";
```

### template literal infers from template literal arguments rejects narrowed result

> Inferred spans preserve union members.

```ds
declare function take<T extends string>(value: `prefix-${T}`): T;

declare let value: `prefix-${"a" | "b"}`;
let result = take(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers constrained spans from call arguments

> Call inference respects constrained template spans.

```ds
declare function take<T extends "a" | "b">(value: `prefix-${T}`): T;

let ok = take("prefix-a");
ok satisfies "a";
```

### template literal rejects call arguments outside constraint

> Constrained spans reject values outside the union.

```ds
declare function take<T extends "a" | "b">(value: `prefix-${T}`): T;

let bad = take("prefix-c");
```

- contains: type "prefix-c" is not assignable to type `prefix-${"a" | "b"}`

### template literal infers from template literal parameters

> Template literal arguments flow into generic spans.

```ds
declare function takeAny<T extends string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
result satisfies `prefix-${"a"}`;
```

### template literal infers from template literal parameters rejects narrowed result

> Inference preserves the full template literal shape.

```ds
declare function takeAny<T extends string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
let bad: "a" = result;
```

- contains: not assignable to type "a"

### template literal infers empty span with literal boundary

> Literal boundaries allow empty captures for call inference.

```ds
declare function take<T>(value: `a${T}`): T;

let ok = take("a");
ok satisfies "";
```
