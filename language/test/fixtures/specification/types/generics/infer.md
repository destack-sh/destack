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

### infer outside conditional extends clauses reports errors

> `infer` declarations are only valid in conditional extends clauses.

```ds
type Invalid = infer U;
```

- infer declarations are only permitted in the extends clause of a conditional type

### inferred reference members reject incompatible values

> Inferred types must satisfy assignments.

```ds
type Box<T> = { value: T };

type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32>> = "no";
```

- type "no" is not assignable to type Inner<Box<int32>>

### infer from nested return references

> `infer` binds inside return type references.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let ok: Item<() => Box<string>> = "ok";
ok satisfies string;
```

### inferred return members reject incompatible values

> Nested return inference rejects incompatible values.

```ds
type Box<T> = { value: T };

type Item<T> = T extends () => Box<infer U> ? U : never;

let bad: Item<() => Box<string>> = 1;
```

- type 1 is not assignable to type Item<() => Box<string>>

### infer from function parameters

> `infer` can bind function parameter types.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let ok: FirstArg<(value: string, count: int32) => void> = "ok";
ok satisfies string;
```

### inferred parameters reject incompatible values

> Parameter inference rejects incompatible values.

```ds
type FirstArg<T> = T extends (value: infer U, count: int32) => void ? U : never;

let bad: FirstArg<(value: string, count: int32) => void> = 1;
```

- type 1 is not assignable to type FirstArg<(value: string, count: int32) => void>

### infer from array element types

> `infer` can bind array element types.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let ok: ElementOf<string[]> = "ok";
ok satisfies string;
```

### inferred array elements reject incompatible values

> Element inference rejects incompatible values.

```ds
type ElementOf<T> = T extends (infer U)[] ? U : never;

let bad: ElementOf<string[]> = 1;
```

- type 1 is not assignable to type ElementOf<string[]>

### infer from object property types

> `infer` can bind property types inside object patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let ok: ValueOf<{ value: boolean }> = true;
ok satisfies boolean;
```

### inferred object properties reject incompatible values

> Property inference rejects incompatible values.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : never;

let bad: ValueOf<{ value: boolean }> = 1;
```

- type 1 is not assignable to type ValueOf<{ value: boolean }>

### infer merges repeated object bindings

> Repeated `infer` bindings merge inferred candidates.

```ds
type Both<T> = T extends { a: infer U, b: infer U } ? U : "no";

let ok: Both<{ a: string, b: string }> = "ok";
let ok2: Both<{ a: string, b: int32 }> = 1;
ok2 satisfies string | number;
```

### repeated object bindings reject incompatible values

> Merged inferences reject incompatible values.

```ds
type Both<T> = T extends { a: infer U, b: infer U } ? U : "no";

let bad: Both<{ a: string, b: int32 }> = true;
```

- type true is not assignable to type Both<{ a: string, b: int32 }>

### infer from multi-parameter functions

> `infer` can bind tuple-like parameter lists from rest parameters.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let ok: Params<(a: string, b: int32) => void> = ["ok", 1];
ok satisfies (string, int32);
```

### inferred parameter tuples reject incompatible values

> Parameter tuple inference rejects incompatible tuples.

```ds
type Params<T> = T extends (...args: infer P) => void ? P : never;

let bad: Params<(a: string, b: int32) => void> = ["ok", "no"];
```

- type (string, "no") is not assignable to type Params<(a: string, b: int32) => void>

### infer merges repeated parameter bindings

> Repeated `infer` bindings intersect contravariant parameter candidates.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let ok: Param<(a: string, b: string) => void> = "ok";
```

### repeated parameter bindings reject incompatible values

> Intersections reject incompatible values.

```ds
type Param<T> = T extends (a: infer U, b: infer U) => void ? U : "no";

let bad: Param<(a: string, b: int32) => void> = "ok";
```

- type "ok" is not assignable to type Param<(a: string, b: int32) => void>

### infer merges repeated tuple bindings

> Repeated `infer` bindings union tuple candidates.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let ok: Pair<(string, string)> = "ok";
let ok2: Pair<(string, int32)> = 1;
ok2 satisfies string | number;
```

### repeated tuple bindings reject incompatible values

> Unioned inferences reject incompatible values.

```ds
type Pair<T> = T extends (infer U, infer U) ? U : "no";

let bad: Pair<(string, int32)> = true;
```

- type true is not assignable to type Pair<(string, int32)>

### infer distributes over unions

> Conditional infer distributes when the left side is a union.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let ok: Inner<Box<int32> | Box<string>> = "ok";
ok satisfies int32 | string;
```

### distributed infer rejects incompatible values

> Distributed inference rejects incompatible values.

```ds
type Box<T> = { value: T };
type Inner<T> = T extends Box<infer U> ? U : never;

let bad: Inner<Box<int32> | Box<string>> = true;
```

- type true is not assignable to type Inner<Box<int32> | Box<string>>

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

- type "a" is not assignable to type NonDist<`foo-a` | `bar-b`>

### infer distributes when unwrapped rejects missing matches

> Distributive inference rejects non matching members.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let badDist: Dist<`foo-a` | `bar-b`> = "b";
```

- type "b" is not assignable to type Dist<`foo-a` | `bar-b`>

### infer does not distribute without type parameters

> Distribution only applies to naked type parameters.

```ds
type NonDistLiteral = (`foo-a` | `bar-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistLiteral = "no";
```

### non-distributive infer rejects distributed matches

> Non type parameter inputs reject the true branch.

```ds
type NonDistLiteral = (`foo-a` | `bar-b`) extends `foo-${infer A}` ? A : "no";

let bad: NonDistLiteral = "a";
```

- type "a" is not assignable to type nondistliteral

### infer merges non distributive union matches

> Non distributive unions merge inferred candidates.

```ds
type NonDistAll = (`foo-a` | `foo-b`) extends `foo-${infer A}` ? A : "no";

let ok: NonDistAll = "a";
let ok2: NonDistAll = "b";
```

### non-distributive union matches reject else values

> Non distributive unions reject the else branch when matches exist.

```ds
type NonDistAll = (`foo-a` | `foo-b`) extends `foo-${infer A}` ? A : "no";

let bad: NonDistAll = "no";
```

- type "no" is not assignable to type nondistall

### infer merges union branch bindings

> Union patterns merge inferred candidates.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let okA: Right<{ a: string }> = "ok";
let okB: Right<{ b: int32 }> = 1;
let okBoth: Right<{ a: string, b: int32 }> = "ok";
let okNone: Right<{ c: boolean }> = "no";
```

### union branch bindings reject incompatible values

> Union patterns reject incompatible values.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let bad: Right<{ a: string, b: int32 }> = true;
```

- type true is not assignable to type Right<{ a: string, b: int32 }>

### unmatched union branches reject true-branch values

> Unmatched unions reject the true branch.

```ds
type Right<T> = T extends ({ a: infer U } | { b: infer U }) ? U : "no";

let badNone: Right<{ c: boolean }> = "ok";
```

- type "ok" is not assignable to type Right<{ c: boolean }>

### infer falls back to else branch

> Conditional infer selects the else branch when the pattern does not match.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let ok: Fallback<string> = 1;
ok satisfies int32;
```

### else-branch infer rejects incompatible values

> Else branch inference rejects incompatible values.

```ds
type Fallback<T> = T extends { value: infer U } ? U : int32;

let bad: Fallback<string> = "no";
```

- type "no" is not assignable to type Fallback<string>

### infer from unknown falls back

> `unknown` does not match structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let ok: ValueOf<unknown> = "no";
```

### unknown fallback rejects true-branch values

> `unknown` rejects the true branch.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<unknown> = 1;
```

- type 1 is not assignable to type ValueOf<unknown>

### infer from never yields never for object patterns

> `never` produces `never` for structural infer patterns.

```ds
type ValueOf<T> = T extends { value: infer U } ? U : "no";

let bad: ValueOf<never> = "no";
```

- type "no" is not assignable to type ValueOf<never>

### infer from never yields never for constrained object patterns

> Constrained structural `infer` still yields `never` for `never` input.

```ds
type ValueOf<T> = T extends { value: infer U extends number } ? U : "no";

let bad: ValueOf<never> = "no";
```

- type "no" is not assignable to type ValueOf<never>

### infer from never yields never for template patterns

> `never` produces `never` in conditional template inference.

```ds
type FromNever = never extends `foo-${infer A}` ? A : "no";

let bad: FromNever = "bar";
```

- type "bar" is not assignable to type fromnever

### infer from never honors constrained template spans

> Constrained spans still yield `never` for `never` input.

```ds
type FromNever = never extends `foo-${infer A extends number | string | boolean}` ? A : "no";

let bad: FromNever = "bar";
```

- type "bar" is not assignable to type fromnever

### infer from constrained template spans

> Constrained template `infer` binds only matching spans.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let ok: FromId<"id-42"> = 42;
ok satisfies 42;
```

### constrained template spans reject incompatible values

> Constrained spans reject the else branch.

```ds
type FromId<T> = T extends `id-${infer A extends number}` ? A : "no";

let bad: FromId<"id-42"> = "no";
```

- type "no" is not assignable to type FromId<"id-42">

### infer distributes over never for type parameters

> Distributive inference over `never` yields `never`.

```ds
type Dist<T> = T extends `foo-${infer A}` ? A : "no";

let bad: Dist<never> = "no";
```

- type "no" is not assignable to type Dist<never>

### non distributive conditionals treat never as a normal type

> Non distributive conditionals treat `never` like any other type.

```ds
type NonDist<T> = [T] extends [string] ? 1 : 2;

type Result = NonDist<never>;

const ok: Result = 1;
ok satisfies 1;
```

### infer from mapped key object patterns

> `infer` can bind inside mapped key object patterns.

```ds
type SearchValue<T> = T extends { [K in "query"]: infer Query } ? Query : never;

let ok: SearchValue<{ query: string }> = "ok";
ok satisfies string;
```

### inferred mapped key values reject incompatible assignments

> Mapped key inference rejects incompatible assignments.

```ds
type SearchValue<T> = T extends { [K in "query"]: infer Query } ? Query : never;

let bad: SearchValue<{ query: string }> = 1;
```

- type 1 is not assignable to type SearchValue<{ query: string }>

### infer in nested conditional clauses

> Nested conditional clauses resolve the nearest inferred type variable.

```ds
type Nested<T> = T extends { value: unknown }
  ? T["value"] extends { inner: infer Inner }
    ? Inner
    : never
  : never;

let ok: Nested<{ value: { inner: int32 } }> = 1;
ok satisfies int32;
```

## literal precision behavior

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

- type string is not assignable to type `id:${"users" | "posts"}`

## cross-module literal precision behavior

### imported generic inference preserves const literal precision

Imported generic calls keep const literal precision at the call site.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import { id } from "./helper";

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
import { id } from "./helper";

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
import { id } from "./helper";

let value = "ready";
let result = id(value);

result satisfies "ready";
```

- type string is not assignable to type `id:${"users" | "posts"}`

### renamed re-export generic inference preserves const literal precision

Renamed re-exports preserve const literal precision at imported call sites.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:index.ds
export { id as identity } from "./helper";
```

```ds:main.ds
import { identity } from "./index";

const value = "ready";
const result = identity(value);

result satisfies "ready";
```

### export-star generic inference keeps let widening behavior

Export-star forwarding keeps mutable-source widening behavior.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:index.ds
export * from "./helper";
```

```ds:main.ds
import { id } from "./index";

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
import * as api from "./helper";

const value = true ? "api" : "admin";
const result = api.id(value);

result satisfies "api" | "admin";
```

### namespace import generic inference keeps let ternary widening behavior

Namespace imports keep mutable ternary widening behavior at generic call sites.

```ds:helper.ds
export function id<T>(value: T): T {
    return value;
}
```

```ds:main.ds
import * as api from "./helper";

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

- type string is not assignable to type `id:${"users" | "posts"}`

## constrained literal inference

### constrained generic inference keeps const literal precision

Const literals satisfies constrained generic parameters with literal precision.

```ts
declare function choose<T extends "dev" | "prod">(value: T): T;

const mode = "dev";
const result = choose(mode);

result satisfies "dev";
```

### constrained generic inference rejects widened let literals

Widened mutable literals do not satisfy constrained literal generic parameters.

```ts
declare function choose<T extends "dev" | "prod">(value: T): T;

let mode = "dev";
choose(mode);
```

- type string is not assignable to type `id:${"users" | "posts"}`

### constrained generic inference keeps const ternary literal unions

Const ternary unions remain precise when inferring constrained generic parameters.

```ts
declare function choose<T extends "dev" | "prod">(value: T): T;

const mode = true ? "dev" : "prod";
const result = choose(mode);

result satisfies "dev" | "prod";
```

### constrained generic inference rejects widened let ternary literals

Mutable ternary literals widen and fail constrained literal generic inference.

```ts
declare function choose<T extends "dev" | "prod">(value: T): T;

let mode = true ? "dev" : "prod";
choose(mode);
```

- type string is not assignable to type `id:${"users" | "posts"}`

## objects and templates

### constrained generic inference keeps const object discriminants

Const object literals preserve discriminants through constrained generic inference.

```ts
declare function select<T extends { kind: "a" | "b" }>(value: T): T;

const value = { kind: "a" as const, payload: 1 };
const result = select(value);

result.kind satisfies "a";
```

### constrained generic inference rejects widened object discriminants

Widened object discriminants fails constrained literal generic inference.

```ts
declare function select<T extends { kind: "a" | "b" }>(value: T): T;

let value = { kind: "a", payload: 1 };
select(value);
```

- contains: not assignable

### constrained template inference keeps const span literals

Template span inference keeps const span literals under constrained generics.

```ts
declare function parse<T extends "users" | "posts">(value: `id:${T}`): T;

const value = "id:users";
const result = parse(value);

result satisfies "users";
```

### constrained template inference rejects widened let strings

Widened mutable strings does not satisfy constrained template span generics.

```ts
declare function parse<T extends "users" | "posts">(value: `id:${T}`): T;

let value = "id:users";
parse(value);
```

- contains: not assignable
