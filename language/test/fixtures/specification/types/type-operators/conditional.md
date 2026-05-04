# Conditional Types

## branching

### conditional types pick true branch

> Conditional types select the matching branch.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<string> = "ok";
```

### conditional types reject the opposite branch

> Conditional types reject values from the other branch.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<string> = 1;
```

- type 1 is not assignable to type Select<string>

### conditional types pick false branch

> Conditional types select the else branch when the match fails.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<int32> = 1;
```

### conditional types reject the true branch for non matches

> Non matching inputs reject the true branch.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<int32> = "no";
```

- type "no" is not assignable to type Select<int32>

### conditional types distribute over unions

> Conditional types distribute over union inputs.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let ok: OnlyStrings<string | int32> = "ok";
```

### conditional types reject non matching union members

> Conditional types filter out non matching union members.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let bad: OnlyStrings<string | int32> = 1;
```

- type 1 is not assignable to type OnlyStrings<string | int32>

### conditional types with unknown select else branch

> `unknown` selects the false branch.

```ds
type Select<T> = T extends string ? "yes" : "no";

let ok: Select<unknown> = "no";
```

### conditional types with unknown reject true branch

> `unknown` rejects the true branch.

```ds
type Select<T> = T extends string ? "yes" : "no";

let bad: Select<unknown> = "yes";
```

- contains: not assignable

### conditional types treat never as empty unions

> `never` yields `never` in conditional types.

```ds
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let bad: Result = "no";
```

- contains: not assignable

### conditional types disable distribution with tuples

> Wrapping types disables distributive behavior.

```ds
type Wrapped<T> = [T] extends [string] ? "yes" : "no";

let ok: Wrapped<string | int32> = "no";
```

## distribution torture

### distributive conditionals preserve both matching branch results

> Distributive conditionals evaluate each union member independently.

```ds
type Dist<T> = T extends "a" ? 1 : 0;

let one: Dist<"a" | "b"> = 1;
let zero: Dist<"a" | "b"> = 0;
```

### non-distributive wrapped conditionals collapse union checks

> Wrapped conditionals evaluate the union as one whole relation.

```ds
type NonDist<T> = [T] extends ["a"] ? 1 : 0;

let ok: NonDist<"a" | "b"> = 0;
```

### non-distributive wrapped conditionals reject distributive branch values

> Wrapped conditionals reject branch values that only exist in distributive evaluation.

```ds
type NonDist<T> = [T] extends ["a"] ? 1 : 0;

let bad: NonDist<"a" | "b"> = 1;
```

- contains: not assignable

### wrapped unknown conditionals choose the false branch

> Wrapped `unknown` does not satisfy narrower true-branch constraints.

```ds
type WrappedUnknown = [unknown] extends [string] ? "yes" : "no";

let ok: WrappedUnknown = "no";
```

## infer extraction

### conditional infer extracts nested generic members

> Conditional `infer` extracts nested generic members from matching shapes.

```ts
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const ok: Unbox<Box<"ready">> = "ready";
```

### conditional infer rejects non matching extracted members

> Extracted conditional members reject incompatible assignments.

```ts
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const bad: Unbox<Box<"ready">> = "no";
```

- contains: not assignable

### conditional infer over unions preserves distributed member unions

> Conditional `infer` distributes over unions and preserves extracted member unions.

```ts
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const ok1: Unbox<Box<"a"> | Box<"b">> = "a";
const ok2: Unbox<Box<"a"> | Box<"b">> = "b";
```

### conditional infer over unions rejects values outside extracted members

> Distributed conditional extraction rejects values outside the extracted member union.

```ts
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;

const bad: Unbox<Box<"a"> | Box<"b">> = "c";
```

- contains: not assignable

### wrapped conditional infer keeps union extraction in one relation

> Wrapping both sides disables distribution and infers one union member relation.

```ts
type Box<T> = { value: T };
type WrappedUnbox<T> = [T] extends [Box<infer U>] ? U : never;

const ok1: WrappedUnbox<Box<"a"> | Box<"b">> = "a";
const ok2: WrappedUnbox<Box<"a"> | Box<"b">> = "b";
```
