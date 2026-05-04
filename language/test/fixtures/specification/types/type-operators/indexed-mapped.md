# Indexed Access Over Mapped Types

## key remapping

### remapped keys support indexed union reads

> Key-remapped mapped types support indexed access over remapped member names.

```ts
type Source = { id: number; name: string };
type Remap<T> = { [K in keyof T as `x_${K & string}`]: T[K] };
type Value = Remap<Source>["x_id" | "x_name"];

const first: Value = 1;
const second: Value = "ok";
```

### remapped keys reject missing indexed members

> Indexed access over remapped keys rejects names that were not produced by the remap.

```ts
type Source = { id: number; name: string };
type Remap<T> = { [K in keyof T as `x_${K & string}`]: T[K] };
type Missing = Remap<Source>["x_missing"];
```

- contains: does not exist

## optional mapped types

### optional mapped types include undefined in indexed reads

> Indexed reads from optional mapped types include `undefined` in the resulting value type.

```ts
type Optional<T> = { [K in keyof T]?: T[K] };
type MaybeName = Optional<{ name: string }>["name"];

const value: MaybeName = undefined;
```

### optional mapped types reject unrelated indexed values

> Optional mapped types reject values outside the member union plus `undefined`.

```ts
type Optional<T> = { [K in keyof T]?: T[K] };
type MaybeName = Optional<{ name: string }>["name"];

const value: MaybeName = 1;
```

- contains: not assignable

## imported alias chains

### imported alias chains preserve indexed mapped precision

> Indexed access through mapped aliases keeps its type through imports and alias chains.

```ts:helper.ts
export type Box<T> = { value: T };
```

```ts:index.ts
export type { Box as RoutedBox } from "./helper";
```

```ts:main.ts
import type { RoutedBox } from "./index";

type Value = RoutedBox<"ok">["value"];

const value: Value = "ok";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```
