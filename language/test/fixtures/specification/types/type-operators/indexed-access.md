# Indexed Access Types

Indexed access types use the `T[K]` type operator.
Fixed arrays use `[T; N]`.

## object access

### indexed access reads property types

```ts
type User = { name: string; age: number };
type Name = User["name"];

const value: Name = "Ada";
value satisfies string;
```

### indexed access rejects missing properties

```ts
type User = { name: string; age: number };
type Missing = User["missing"];
```

- contains: does not exist

### numeric keys on object types stay indexed access

```ds
type Pair = { 0: string, 1: int32 };
type Right = Pair[1];

declare const value: Right;
value satisfies int32;
```

### missing numeric object keys are rejected

```ds
type ObjectLike = { label: string };
type Missing = ObjectLike[5];
```

- contains: does not exist

## unions

### indexed access distributes across unions

```ts
type A = { kind: "a"; value: number };
type B = { kind: "b"; value: string };
type Value = (A | B)["value"];

const first: Value = 1;
const second: Value = "hi";
```

### indexed access with key unions yields unioned values

```ts
type User = { name: string; age: number };
type Value = User["name" | "age"];

const first: Value = "Ada";
const second: Value = 42;
```

### indexed access rejects values outside selected members

```ts
type User = { name: string; age: number };
type Value = User["name" | "age"];

const bad: Value = true;
```

- contains: not assignable

## generics

### generic indexed access requires a key constraint

```ds
type ValueAt<T, K: keyof T> = T[K];
type User = { name: string, age: int32 };

declare const value: ValueAt<User, "name">;
value satisfies string;
```

### unconstrained generic indexed access is rejected

```ds
type ValueAt<T, K> = T[K];
```

- contains: index

## tuples and arrays

### tuple indexing keeps indexed access semantics

```ds
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;
first satisfies string;
```

### dynamic array indexing yields element types

```ts
type Element<T extends string[]> = T[number];

declare const value: Element<["a", "b"]>;
value satisfies string;
```

### fixed array syntax is separate

```ds
type Lane<comptime N: uint> = [uint8; N];

declare const value: Lane<4>;
value satisfies [uint8; 4];
```

## optional unions

### indexed access over optional union members includes undefined

Indexed access over optional members in a union includes `undefined` in the resulting value type.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const maybe: Value = undefined;
maybe satisfies number | string | undefined;
```

### indexed access over optional union members rejects assignment to missing required value

Those optional indexed reads reject assignment into required-only target reads.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const maybe: Value = undefined;
maybe satisfies number | string;
```

- contains: not assignable

### indexed access over optional union members rejects unrelated values

Optional-union indexed reads also reject values outside the member union.

```ts
type Input =
    | { kind: "a", value?: number }
    | { kind: "b", value: string };

type Value = Input["value"];

const bad: Value = true;
```

- contains: not assignable
