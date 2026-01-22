# Conditional Infer

## infer

### infer from type reference patterns

> `infer` binds type arguments inside reference patterns.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32>> = 1;
ok satisfies int32;
```

### infer from type reference patterns rejects mismatches

> Inferred types must satisfy assignments.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32>> = "no";
```

- contains: type "no" is not assignable to type inner<<type>>

### infer from function return types

> `infer` binds return types inside function patterns.

```ds
type ReturnOf<T> = T extends (...args: any[]) => infer R ? R : never;

let ok: ReturnOf<() => string> = "ok";
ok satisfies string;
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

### infer from function return types rejects mismatches

> Return inference rejects incompatible values.

```ds
type ReturnOf<T> = T extends (...args: any[]) => infer R ? R : never;

let bad: ReturnOf<() => string> = 1;
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

- contains: type 1 is not assignable to type returnof<<type>>

### infer from nested return references

> `infer` binds inside return type references.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let ok: Item<() => Box<string>> = "ok";
ok satisfies string;
```

### infer from nested return references rejects mismatches

> Nested return inference rejects incompatible values.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let bad: Item<() => Box<string>> = 1;
```

- contains: type 1 is not assignable to type item<<type>>

### infer from function parameters

> `infer` can bind function parameter types.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let ok: FirstArg<(value: string, count: int32) => void> = "ok";
ok satisfies string;
```

### infer from function parameters rejects mismatches

> Parameter inference rejects incompatible values.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let bad: FirstArg<(value: string, count: int32) => void> = 1;
```

- contains: type 1 is not assignable to type firstarg<<type>>

### infer from array element types

> `infer` can bind array element types.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let ok: ElementOf<string[]> = "ok";
ok satisfies string;
```

### infer from array element types rejects mismatches

> Element inference rejects incompatible values.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let bad: ElementOf<string[]> = 1;
```

- contains: type 1 is not assignable to type elementof<<type>>

### infer from object property types

> `infer` can bind property types inside object patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let ok: ValueOf<{ value: boolean }> = true;
ok satisfies boolean;
```

### infer from object property types rejects mismatches

> Property inference rejects incompatible values.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let bad: ValueOf<{ value: boolean }> = 1;
```

- contains: type 1 is not assignable to type valueof<<type>>

### infer merges repeated object bindings

> Repeated `infer` bindings merge inferred candidates.

```ds
type Both<T> = T extends { a: infer U, b: infer U } ? U : "no";

let ok: Both<{ a: string, b: string }> = "ok";
let ok2: Both<{ a: string, b: int32 }> = 1;
ok2 satisfies string | number;
```

### infer merges repeated object bindings rejects mismatches

> Merged inferences reject incompatible values.

```ds
type Both<T> = T extends { a: infer U, b: infer U } ? U : "no";

let bad: Both<{ a: string, b: int32 }> = true;
```

- contains: type true is not assignable to type both<<type>>

### infer from multi-parameter functions

> `infer` can bind tuple-like parameter lists from rest parameters.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let ok: Params<(a: string, b: int32) => void> = ["ok", 1];
ok satisfies (string, int32);
```

### infer from multi-parameter functions rejects mismatches

> Parameter tuple inference rejects incompatible tuples.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let bad: Params<(a: string, b: int32) => void> = ["ok", "no"];
```

- contains: type (string, "no") is not assignable to type params<<type>>

### infer merges repeated parameter bindings

> Repeated `infer` bindings intersect contravariant parameter candidates.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let ok: Param<(a: string, b: string) => void> = "ok";
```

### infer merges repeated parameter bindings rejects mismatches

> Intersections reject incompatible values.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let bad: Param<(a: string, b: int32) => void> = "ok";
```

- contains: type "ok" is not assignable to type param<<type>>

### infer merges repeated tuple bindings

> Repeated `infer` bindings union tuple candidates.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let ok: Pair<(string, string)> = "ok";
let ok2: Pair<(string, int32)> = 1;
ok2 satisfies string | number;
```

### infer merges repeated tuple bindings rejects mismatches

> Unioned inferences reject incompatible values.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let bad: Pair<(string, int32)> = true;
```

- contains: type true is not assignable to type pair<<type>>

### infer distributes over unions

> Conditional infer distributes when the left side is a union.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32> | Box<string>> = "ok";
ok satisfies int32 | string;
```

### infer distributes over unions rejects mismatches

> Distributed inference rejects incompatible values.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32> | Box<string>> = true;
```

- contains: type true is not assignable to type inner<<type>>

### infer does not distribute when wrapped

> Wrapping the type parameter prevents distributive inference.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";
type NonDist<T> = [T] extends [`foo-${infer A}`] ? A : "no";

let okDist: Dist<`foo-a` | `bar-b`> = "a";
let okDist2: Dist<`foo-a` | `bar-b`> = "no";
let okNon: NonDist<`foo-a` | `bar-b`> = "no";
```

### infer does not distribute when wrapped rejects distributive values

> Wrapped conditionals do not accept distributed matches.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";
type NonDist<T> = [T] extends [`foo-${infer A}`] ? A : "no";

let badNon: NonDist<`foo-a` | `bar-b`> = "a";
```

- contains: type "a" is not assignable to type nondist<<type>>

### infer distributes when unwrapped rejects missing matches

> Distributive inference rejects non matching members.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let badDist: Dist<`foo-a` | `bar-b`> = "b";
```

- contains: type "b" is not assignable to type dist<<type>>

### infer does not distribute without type parameters

> Distribution only applies to naked type parameters.

```ds
type NonDistLiteral = (`foo-a` | `bar-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistLiteral = "no";
```

### infer does not distribute without type parameters rejects matches

> Non type parameter inputs reject the true branch.

```ds
type NonDistLiteral = (`foo-a` | `bar-b`) extends `foo-${infer A}` ? A : "no";

let bad: NonDistLiteral = "a";
```

- contains: type "a" is not assignable to type nondistliteral

### infer merges non distributive union matches

> Non distributive unions merge inferred candidates.

```ds
type NonDistAll = (`foo-a` | `foo-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistAll = "a";
let ok2: NonDistAll = "b";
```

### infer merges non distributive union matches rejects else

> Non distributive unions reject the else branch when matches exist.

```ds
type NonDistAll = (`foo-a` | `foo-b`) extends `foo-${infer A}` ? A : "no";

let bad: NonDistAll = "no";
```

- contains: type "no" is not assignable to type nondistall

### infer merges union branch bindings

> Union patterns merge inferred candidates.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let okA: Right<{ a: string }> = "ok";
let okB: Right<{ b: int32 }> = 1;
let okBoth: Right<{ a: string, b: int32 }> = "ok";
let okNone: Right<{ c: boolean }> = "no";
```

### infer merges union branch bindings rejects mismatches

> Union patterns reject incompatible values.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let bad: Right<{ a: string, b: int32 }> = true;
```

- contains: type true is not assignable to type right<<type>>

### infer merges union branch bindings rejects else when unmatched

> Unmatched unions reject the true branch.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let badNone: Right<{ c: boolean }> = "ok";
```

- contains: type "ok" is not assignable to type right<<type>>

### infer falls back to else branch

> Conditional infer selects the else branch when the pattern does not match.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let ok: Fallback<string> = 1;
ok satisfies int32;
```

### infer falls back to else branch rejects mismatches

> Else branch inference rejects incompatible values.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let bad: Fallback<string> = "no";
```

- contains: type "no" is not assignable to type fallback<<type>>

### infer from any yields any

> `any` produces an `any` inferred result.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

declare let value: ValueOf<any>;
let okString: string = value;
let okNumber: number = value;
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

### infer from unknown falls back

> `unknown` does not match structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let ok: ValueOf<unknown> = "no";
```

### infer from unknown falls back rejects mismatches

> `unknown` rejects the true branch.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<unknown> = 1;
```

- contains: type 1 is not assignable to type valueof<<type>>

### infer from never yields never for object patterns

> `never` produces `never` for structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<never> = "no";
```

- contains: type "no" is not assignable to type valueof<<type>>

### infer from never yields never for constrained object patterns

> Constrained structural `infer` still yields `never` for `never` input.

```ds
type ValueOf<T> = T extends { value: infer U extends number } ? U : "no";

let bad: ValueOf<never> = "no";
```

- contains: type "no" is not assignable to type valueof<<type>>

### infer from never yields never for template patterns

> `never` produces `never` in conditional template inference.

```ds
type FromNever = never extends `foo-${infer A}` ? A : "no";

let bad: FromNever = "bar";
```

- contains: type "bar" is not assignable to type fromnever

### infer from never honors constrained template spans

> Constrained spans still yield `never` for `never` input.

```ds
type FromNever = never extends `foo-${infer A extends number | string | boolean}` ? A : "no";

let bad: FromNever = "bar";
```

- contains: type "bar" is not assignable to type fromnever

### infer from constrained template spans

> Constrained template `infer` binds only matching spans.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let ok: FromId<"id-42"> = 42;
ok satisfies 42;
```

### infer from constrained template spans rejects mismatches

> Constrained spans reject the else branch.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let bad: FromId<"id-42"> = "no";
```

- contains: type "no" is not assignable to type fromid<<type>>

### infer distributes over never for type parameters

> Distributive inference over `never` yields `never`.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let bad: Dist<never> = "no";
```

- contains: type "no" is not assignable to type dist<<type>>
