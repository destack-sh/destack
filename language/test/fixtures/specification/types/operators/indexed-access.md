# Indexed Access Types

## basic access

### indexed access reads property types

> Indexed access yields the property type for known keys.

```ts
type User = { name: string; age: number };
type Name = User["name"];

const value: Name = "Ada";
value satisfies string;
```

### indexed access rejects missing properties

> Missing properties in indexed access are rejected.

```ts
type User = { name: string; age: number };
type Missing = User["missing"];
```

- contains: does not exist

## unions and intersections

### indexed access distributes across unions

> Indexed access distributes across union members.

```ts
type A = { kind: "a"; value: number };
type B = { kind: "b"; value: string };
type Value = (A | B)["value"];

const ok1: Value = 1;
const ok2: Value = "hi";
```

### indexed access rejects non union members

> Indexed access rejects values outside the unioned member types.

```ts
type A = { kind: "a"; value: number };
type B = { kind: "b"; value: string };
type Value = (A | B)["value"];

const bad: Value = true;
```

- contains: not assignable

### indexed access on intersections preserves members

> Indexed access on intersections preserves property types.

```ts
type Left = { name: string };
type Right = { name: "Ada" };
type Name = (Left & Right)["name"];

const ok: Name = "Ada";
```

### indexed access with key unions yields unioned member values

> Indexed access with key unions yields the union of selected property value types.

```ts
type User = { name: string; age: number };
type Value = User["name" | "age"];

const ok1: Value = "Ada";
const ok2: Value = 42;
```

### indexed access with key unions rejects non member value types

> Indexed access with key unions rejects values outside the selected member value union.

```ts
type User = { name: string; age: number };
type Value = User["name" | "age"];

const bad: Value = true;
```

- contains: not assignable

## indexed access with conditionals

### indexed access distributes through conditional unions

> Conditional unions preserve indexed access unions.

```ts
type A = { value: number };
type B = { value: string };
type Values<T> = T extends unknown ? T["value"] : never;

const ok: Values<A | B> = 1;
const ok2: Values<A | B> = "hi";
```

### indexed access rejects incompatible conditional values

> Conditional indexed access rejects values outside the union.

```ts
type A = { value: number };
type B = { value: string };
type Values<T> = T extends unknown ? T["value"] : never;

const bad: Values<A | B> = true;
```

- contains: not assignable

### generic indexed access preserves key constrained member types

> Generic indexed access preserves key-constrained member types through helper aliases.

```ts
type ValueOf<T, K extends keyof T> = T[K];
type User = { name: string; age: number };

const ok: ValueOf<User, "name"> = "Ada";
```

### generic indexed access rejects incompatible key constrained member values

> Generic indexed access rejects incompatible values for constrained member selections.

```ts
type ValueOf<T, K extends keyof T> = T[K];
type User = { name: string; age: number };

const bad: ValueOf<User, "age"> = "Ada";
```

- contains: not assignable

## index signatures

### indexed access uses index signatures

> Indexed access yields the index signature type.

```ts
type Bag = { [key: string]: number };
type Value = Bag[string];

const ok: Value = 1;
```

### indexed access rejects incompatible index values

> Index signatures still enforce assignability.

```ts
type Bag = { [key: string]: number };
type Value = Bag[string];

const bad: Value = "no";
```

- contains: not assignable

## .ds disambiguation

### .ds type index uses indexed access when index semantics are admissible

> In `.ds`, `T[N]` keeps indexed-access semantics when normal type-index rules are admissible.

```ds
type Head<T: (string, int32)> = T[0];

declare const value: Head<(string, int32)>;
value satisfies string;
```

### .ds type index can force fixed-size arrays with as comptime

> In `.ds`, `T[N as comptime]` forces fixed-size array construction.

```ds
type Lane<comptime N: number> = uint8[N as comptime];

declare const value: Lane<4>;
value satisfies uint8[4];
```

### .ds prefers indexed access when numeric keys are admissible

> In `.ds`, admissible numeric index access should win over fixed-array interpretation.

```ds
type Pair = { 0: string, 1: number };
type Second = Pair[1];

declare const value: Second;
value satisfies number;
```

### .ds rejects numeric object indexing and does not reinterpret as fixed arrays

> Numeric object indexing should stay indexed-access semantics and reject missing keys.

```ds
type ObjectLike = { label: string };
type Missing = ObjectLike[5];
```

- contains: does not exist

### .ds tuple indexing keeps indexed-access semantics with numeric literals

> Tuple indexing with numeric literals should resolve as indexed access.

```ds
type Pair = (string, int32);
type First = Pair[0];

declare const first: First;
first satisfies string;
```

### .ds type index rejects ambiguous index spaces without as comptime

> In `.ds`, `T[N]` is rejected when index space is ambiguous.

```ds
type Ambiguous<T, N> = T[N];
```

- contains: ambiguous

### .ds as comptime disambiguates numeric static parameters for fixed arrays

> In `.ds`, `as comptime` disambiguates numeric static parameters toward fixed-array lengths.

```ds
type Buffer<comptime N: number> = uint8[N as comptime];

declare const value: Buffer<16>;
value satisfies uint8[16];
```

### .ts type index remains indexed access and never becomes fixed-size arrays

> In `.ts`, `T[N]` always follows TypeScript indexed-access semantics.

```ts
type Element<T extends string[]> = T[number];

declare const value: Element<["a", "b"]>;
value satisfies string;
```

## associated comptime interaction

### associated comptime projections can drive fixed arrays with as comptime

> Associated comptime projections should be valid fixed-array lengths when disambiguated with `as comptime`.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

type Lane = uint8[Segment<string>.Width as comptime];

declare const value: Lane;
value satisfies uint8[8];
```

### unresolved associated comptime projections are rejected in fixed arrays

> Generic unresolved associated comptime projections should be rejected in fixed-array lengths.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

type Lane<Row> = uint8[Segment<Row>.Width as comptime];
```

- contains: static expression

## cross-module disambiguation

### imported associated comptime lengths infer fixed arrays when indexed access is inadmissible

> Imported associated comptime lengths should infer fixed arrays without `as comptime` when indexed access is inadmissible.

```ds:layout.ds
export interface LaneLayout<T> {
    comptime const Count: number = 8;
    type Lane = T[this.Count];
}

export class ByteLaneLayout implements LaneLayout<uint8> {}
```

```ds:main.ds
import { ByteLaneLayout } from "./layout";

declare const lane: ByteLaneLayout.Lane;
lane satisfies uint8[8];
```

### imported associated comptime lengths require as comptime when indexed access is admissible

> Imported associated comptime lengths should require `as comptime` to force fixed-array construction when indexed access is admissible.

```ds:layout.ds
export interface WindowLayout<T> {
    comptime const Count: number = 4;
    type Sample = T[this.Count];
    type Window = T[this.Count as comptime];
}

export class ByteWindowLayout implements WindowLayout<uint8[]> {}
```

```ds:main.ds
import { ByteWindowLayout } from "./layout";

declare const sample: ByteWindowLayout.Sample;
sample satisfies uint8;

declare const window: ByteWindowLayout.Window;
window satisfies uint8[][4 as comptime];
```

### namespace imports preserve fixed-array disambiguation with as comptime

> Namespace imports should preserve fixed-array disambiguation for associated comptime lengths.

```ds:layout.ds
export class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}
```

```ds:main.ds
import * as api from "./layout";

type Lane<Row> = uint8[api.Segment<Row>.Width as comptime];

declare const lane: Lane<string>;
lane satisfies uint8[8];
```

### borrowed references preserve fixed-array disambiguation with imported associated lengths

> Borrowed references should preserve fixed-array disambiguation with imported associated comptime lengths.

```ds:layout.ds
export class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}
```

```ds:main.ds
import { Segment } from "./layout";

function read<Row>(value: &uint8[Segment<Row>.Width as comptime]): uint8 {
    value[0]
}
```

### nested alias chains preserve fixed-array disambiguation with as comptime

> Nested alias chains should preserve fixed-array disambiguation when `as comptime` is explicit.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

type WidthOf<Row> = Segment<Row>.Width;
type Lane<Row> = uint8[WidthOf<Row> as comptime];

declare const lane: Lane<string>;
lane satisfies uint8[8];
```

### nested alias chains prefer indexed access semantics without as comptime

> Nested alias chains should stay in indexed-access mode when `as comptime` is omitted in admissible index contexts.

```ds
class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}

type WidthOf<Row> = Segment<Row>.Width;
type Lane<Row> = uint8[][WidthOf<Row>];

declare const lane: Lane<string>;
lane satisfies uint8;
```

### generic indexed access disambiguation remains stable through helper aliases

> Generic helper aliases should preserve indexed-access semantics when index admissibility holds.

```ds
type ValueAt<T, K: keyof T> = T[K];
type User = { name: string, age: int32 };

declare const value: ValueAt<User, "name">;
value satisfies string;
```
