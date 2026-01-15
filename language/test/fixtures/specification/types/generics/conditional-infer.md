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

## infer from multi-parameter functions

> `infer` can bind tuple-like parameter lists.

```ds
type Params<T> = T extends (args: infer P) => void ? P : never;

let ok: Params<(a: string, b: int32) => void> = ["ok", 1];
let bad: Params<(a: string, b: int32) => void> = ["ok", "no"];
ok satisfies (string, int32);
```

- contains: type string is not assignable to type int32

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

## infer falls back to else branch

> Conditional infer selects the else branch when the pattern does not match.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let ok: Fallback<string> = 1;
let bad: Fallback<string> = "no";
ok satisfies int32;
```

- contains: type string is not assignable to type int32
