# Conditional Infer

`infer` binds components of a matched type inside conditionals.

## infer

### infer from type reference patterns

`infer` binds type arguments inside reference patterns.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32>> = 1;
ok satisfies int32;
```

### infer outside conditional extends clauses reports errors

`infer` declarations are only valid in conditional extends clauses.

```ds
type Invalid = infer U;
```

- contains: infer declarations are only permitted in the extends clause of a conditional type

### inferred reference members reject incompatible values

Inferred types must satisfy assignments.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32>> = "no";
```

- contains: not assignable

### infer from nested return references

`infer` binds inside return type references.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let ok: Item<() => Box<string>> = "ok";
ok satisfies string;
```

### inferred return members reject incompatible values

Nested return inference rejects incompatible values.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let bad: Item<() => Box<string>> = 1;
```

- contains: not assignable

### infer from function parameters

`infer` can bind function parameter types.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let ok: FirstArg<(value: string, count: int32) => void> = "ok";
ok satisfies string;
```

### inferred parameters reject incompatible values

Parameter inference rejects incompatible values.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let bad: FirstArg<(value: string, count: int32) => void> = 1;
```

- contains: not assignable

### infer from array element types

`infer` can bind array element types.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let ok: ElementOf<string[]> = "ok";
ok satisfies string;
```

### inferred array elements reject incompatible values

Element inference rejects incompatible values.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let bad: ElementOf<string[]> = 1;
```

- contains: not assignable

### infer from object property types

`infer` can bind property types inside object patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let ok: ValueOf<type { value: boolean }> = true;
ok satisfies boolean;
```

### inferred object properties reject incompatible values

Property inference rejects incompatible values.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let bad: ValueOf<type { value: boolean }> = 1;
```

- contains: not assignable

### infer merges repeated object bindings

Repeated `infer` bindings merge inferred candidates.

```ds
type Both<T> = T extends { a: infer U; b: infer U } ? U : "no";

let ok: Both<type { a: string; b: string }> = "ok";
let ok2: Both<type { a: string; b: int32 }> = 1;
ok2 satisfies string | number;
```

### repeated object bindings reject incompatible values

Merged inferences reject incompatible values.

```ds
type Both<T> = T extends { a: infer U; b: infer U } ? U : "no";

let bad: Both<type { a: string; b: int32 }> = true;
```

- contains: not assignable

### infer from multi-parameter functions

`infer` can bind tuple-like parameter lists from rest parameters.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let ok: Params<(a: string, b: int32) => void> = ["ok", 1];
ok satisfies (string, int32);
```

### inferred parameter tuples reject incompatible values

Parameter tuple inference rejects incompatible tuples.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let bad: Params<(a: string, b: int32) => void> = ["ok", "no"];
```

- contains: not assignable

### infer merges repeated parameter bindings

Repeated `infer` bindings intersect contravariant parameter candidates.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let ok: Param<(a: string, b: string) => void> = "ok";
```

### repeated parameter bindings reject incompatible values

Intersections reject incompatible values.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let bad: Param<(a: string, b: int32) => void> = "ok";
```

- contains: not assignable

### infer merges repeated tuple bindings

Repeated `infer` bindings union tuple candidates.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let ok: Pair<(string, string)> = "ok";
let ok2: Pair<(string, int32)> = 1;
ok2 satisfies string | number;
```

### repeated tuple bindings reject incompatible values

Unioned inferences reject incompatible values.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let bad: Pair<(string, int32)> = true;
```

- contains: not assignable

### infer distributes over unions

Conditional infer distributes when the left side is a union.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32> | Box<string>> = "ok";
ok satisfies int32 | string;
```

### distributed infer rejects incompatible values

Distributed inference rejects incompatible values.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32> | Box<string>> = true;
```

- contains: not assignable

### infer does not distribute when wrapped

Wrapping the type parameter prevents distributive inference.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";
type NonDist<T> = (T,) extends (`foo-${infer A}`,) ? A : "no";

let okDist: Dist<`foo-a` | `bar-b`> = "a";
let okDist2: Dist<`foo-a` | `bar-b`> = "no";
let okNon: NonDist<`foo-a` | `bar-b`> = "no";
```

### infer does not distribute when wrapped rejects distributive values

Wrapped conditionals do not accept distributed matches.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";
type NonDist<T> = (T,) extends (`foo-${infer A}`,) ? A : "no";

let badNon: NonDist<`foo-a` | `bar-b`> = "a";
```

- contains: not assignable

### infer distributes when unwrapped rejects missing matches

Distributive inference rejects non matching members.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let badDist: Dist<`foo-a` | `bar-b`> = "b";
```

- contains: not assignable

### infer does not distribute without type parameters

Distribution only applies to naked type parameters.

```ds
type NonDistLiteral = `foo-a` | `bar-b` extends `foo-${infer A}` ? A : "no";

let ok: NonDistLiteral = "no";
```

### non-distributive infer rejects distributed matches

Non type parameter inputs reject the true branch.

```ds
type NonDistLiteral = `foo-a` | `bar-b` extends `foo-${infer A}` ? A : "no";

let bad: NonDistLiteral = "a";
```

- contains: not assignable

### infer merges non-distributive union matches

Non-distributive unions merge inferred candidates.

```ds
type NonDistAll = `foo-a` | `foo-b` extends `foo-${infer A}` ? A : "no";

let ok: NonDistAll = "a";
let ok2: NonDistAll = "b";
```

### non-distributive union matches reject else values

Non-distributive unions reject the else branch when matches exist.

```ds
type NonDistAll = `foo-a` | `foo-b` extends `foo-${infer A}` ? A : "no";

let bad: NonDistAll = "no";
```

- contains: not assignable

### infer merges union branch bindings

Union patterns merge inferred candidates.

```ds
type Right<T> = T extends { a: infer U } | { b: infer U } ? U : "no";

let okA: Right<type { a: string }> = "ok";
let okB: Right<type { b: int32 }> = 1;
let okBoth: Right<type { a: string; b: int32 }> = "ok";
let okNone: Right<type { c: boolean }> = "no";
```

### union branch bindings reject incompatible values

Union patterns reject incompatible values.

```ds
type Right<T> = T extends { a: infer U } | { b: infer U } ? U : "no";

let bad: Right<type { a: string; b: int32 }> = true;
```

- contains: not assignable

### unmatched union branches reject true-branch values

Unmatched unions reject the true branch.

```ds
type Right<T> = T extends { a: infer U } | { b: infer U } ? U : "no";

let badNone: Right<type { c: boolean }> = "ok";
```

- contains: not assignable

### infer falls back to else branch

Conditional infer selects the else branch when the pattern does not match.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let ok: Fallback<string> = 1;
ok satisfies int32;
```

### else-branch infer rejects incompatible values

Else branch inference rejects incompatible values.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let bad: Fallback<string> = "no";
```

- contains: not assignable

### infer from unknown falls back

`unknown` does not match structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let ok: ValueOf<unknown> = "no";
```

### unknown fallback rejects true-branch values

`unknown` rejects the true branch.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<unknown> = 1;
```

- contains: not assignable

### infer from never yields never for object patterns

`never` produces `never` for structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<never> = "no";
```

- contains: not assignable

### infer from never yields never for constrained object patterns

Constrained structural `infer` still yields `never` for `never` input.

```ds
type ValueOf<T> = T extends { value: infer U extends number } ? U : "no";

let bad: ValueOf<never> = "no";
```

- contains: not assignable

### infer from never yields never for template patterns

`never` produces `never` in conditional template inference.

```ds
type FromNever = never extends `foo-${infer A}` ? A : "no";

let bad: FromNever = "bar";
```

- contains: not assignable

### infer from never honors constrained template spans

Constrained spans still yield `never` for `never` input.

```ds
type FromNever = never extends `foo-${infer A extends number | string | boolean}` ? A : "no";

let bad: FromNever = "bar";
```

- contains: not assignable

### infer from constrained template spans

Constrained template `infer` binds only matching spans.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let ok: FromId<"id-42"> = 42;
ok satisfies 42;
```

### constrained template spans reject incompatible values

Constrained spans reject the else branch.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let bad: FromId<"id-42"> = "no";
```

- contains: not assignable

### infer distributes over never for type parameters

Distributive inference over `never` yields `never`.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let bad: Dist<never> = "no";
```

- contains: not assignable

### non-distributive conditionals keep never

Tuple-wrapped conditionals do not distribute over `never`.

```ds
type NonDist<T> = (T,) extends (string,) ? 1 : 2;

type Result = NonDist<never>;

const ok: Result = 1;
ok satisfies 1;
```

### infer from mapped key object patterns

`infer` can bind inside mapped key object patterns.

```ds
type SearchValue<T> = T extends { [K in "query"]: infer Query } ? Query : never;

let ok: SearchValue<type { query: string }> = "ok";
ok satisfies string;
```

### inferred mapped key values reject incompatible assignments

Mapped key inference rejects incompatible assignments.

```ds
type SearchValue<T> = T extends { [K in "query"]: infer Query } ? Query : never;

let bad: SearchValue<type { query: string }> = 1;
```

- contains: not assignable

### infer in nested conditional clauses

Nested conditional clauses resolve the nearest inferred type variable.

```ds
type Nested<T> = T extends { value: unknown }
    ? T["value"] extends { inner: infer Inner }
        ? Inner
        : never
    : never;

let ok: Nested<type { value: { inner: int32 } }> = 1;
ok satisfies int32;
```
