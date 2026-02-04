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

## indexed access with conditionals

### indexed access distributes through conditional unions

> Conditional unions preserve indexed access unions.

```ts
type A = { value: number };
type B = { value: string };
type Values<T> = T extends any ? T["value"] : never;

const ok: Values<A | B> = 1;
const ok2: Values<A | B> = "hi";
```

### indexed access rejects incompatible conditional values

> Conditional indexed access rejects values outside the union.

```ts
type A = { value: number };
type B = { value: string };
type Values<T> = T extends any ? T["value"] : never;

const bad: Values<A | B> = true;
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
