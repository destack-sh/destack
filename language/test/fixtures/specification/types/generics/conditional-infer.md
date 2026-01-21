# Conditional Infer

## infer from type reference patterns

> `infer` binds type arguments inside reference patterns.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32>> = 1;
let bad: Inner<Box<int32>> = "no";
ok satisfies int32;
```

- contains: type string is not assignable to type int32

## infer from function return types

> `infer` binds return types inside function patterns.

```ds
type ReturnOf<T> = T extends (...args: any[]) => infer R ? R : never;

let ok: ReturnOf<() => string> = "ok";
let bad: ReturnOf<() => string> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## infer from nested return references

> `infer` binds inside return type references.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let ok: Item<() => Box<string>> = "ok";
let bad: Item<() => Box<string>> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## infer from function parameters

> `infer` can bind function parameter types.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let ok: FirstArg<(value: string, count: int32) => void> = "ok";
let bad: FirstArg<(value: string, count: int32) => void> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## infer from array element types

> `infer` can bind array element types.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let ok: ElementOf<string[]> = "ok";
let bad: ElementOf<string[]> = 1;
ok satisfies string;
```

- contains: type int32 is not assignable to type string

## infer from object property types

> `infer` can bind property types inside object patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let ok: ValueOf<{ value: boolean }> = true;
let bad: ValueOf<{ value: boolean }> = 1;
ok satisfies boolean;
```

- contains: type int32 is not assignable to type boolean

## infer merges repeated object bindings

> Repeated `infer` bindings merge inferred candidates.

```ds
type Both<T> = T extends { a: infer U, b: infer U } ? U : "no";

let ok: Both<{ a: string, b: string }> = "ok";
let ok2: Both<{ a: string, b: int32 }> = 1;
let bad: Both<{ a: string, b: int32 }> = true;
ok2 satisfies string | number;
```

- contains: type boolean is not assignable to type string | number

## infer from multi-parameter functions

> `infer` can bind tuple-like parameter lists.

```ds
type Params<T> = T extends (args: infer P) => void ? P : never;

let ok: Params<(a: string, b: int32) => void> = ["ok", 1];
let bad: Params<(a: string, b: int32) => void> = ["ok", "no"];
ok satisfies (string, int32);
```

- contains: type string is not assignable to type int32

## infer merges repeated parameter bindings

> Repeated `infer` bindings intersect contravariant parameter candidates.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let ok: Param<(a: string, b: string) => void> = "ok";
let bad: Param<(a: string, b: int32) => void> = "ok";
```

- contains: type string is not assignable to type never

## infer merges repeated tuple bindings

> Repeated `infer` bindings union tuple candidates.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let ok: Pair<(string, string)> = "ok";
let ok2: Pair<(string, int32)> = 1;
let bad: Pair<(string, int32)> = true;
ok2 satisfies string | number;
```

- contains: type boolean is not assignable to type string | number

## infer distributes over unions

> Conditional infer distributes when the left side is a union.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32> | Box<string>> = "ok";
let bad: Inner<Box<int32> | Box<string>> = true;
ok satisfies int32 | string;
```

- contains: type boolean is not assignable to type int32 | string

## infer does not distribute when wrapped

> Wrapping the type parameter prevents distributive inference.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";
type NonDist<T> = [T] extends [`foo-${infer A}`] ? A : "no";

let okDist: Dist<`foo-a` | `bar-b`> = "a";
let okDist2: Dist<`foo-a` | `bar-b`> = "no";
let badDist: Dist<`foo-a` | `bar-b`> = "b";

let okNon: NonDist<`foo-a` | `bar-b`> = "no";
let badNon: NonDist<`foo-a` | `bar-b`> = "a";
```

- contains: type string is not assignable to type `a` | `no`
- contains: type string is not assignable to type `no`

## infer does not distribute without type parameters

> Distribution only applies to naked type parameters.

```ds
type NonDistLiteral = (`foo-a` | `bar-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistLiteral = "no";
let bad: NonDistLiteral = "a";
```

- contains: type string is not assignable to type `no`

## infer merges non distributive union matches

> Non distributive unions merge inferred candidates.

```ds
type NonDistAll = (`foo-a` | `foo-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistAll = "a";
let ok2: NonDistAll = "b";
let bad: NonDistAll = "no";
```

- contains: type `no` is not assignable to type `a` | `b`

## infer merges union branch bindings

> Union patterns merge inferred candidates.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let okA: Right<{ a: string }> = "ok";
let okB: Right<{ b: int32 }> = 1;
let okBoth: Right<{ a: string, b: int32 }> = "ok";
let bad: Right<{ a: string, b: int32 }> = true;
let okNone: Right<{ c: boolean }> = "no";
let badNone: Right<{ c: boolean }> = "ok";
```

- contains: type boolean is not assignable to type string | number
- contains: type string is not assignable to type `no`

## infer falls back to else branch

> Conditional infer selects the else branch when the pattern does not match.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let ok: Fallback<string> = 1;
let bad: Fallback<string> = "no";
ok satisfies int32;
```

- contains: type string is not assignable to type int32

## infer from any yields unknown

> `any` infers `unknown` for structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

declare let value: ValueOf<any>;
let bad: string = value;
```

- contains: type unknown is not assignable to type string

## infer from unknown falls back

> `unknown` does not match structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let ok: ValueOf<unknown> = "no";
let bad: ValueOf<unknown> = 1;
```

- contains: type int32 is not assignable to type `no`
