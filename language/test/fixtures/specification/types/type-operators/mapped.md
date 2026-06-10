# Mapped Types

Mapped types iterate over static key unions.

## keys

### mapped types use instantiated keys

Mapping sees the actual instantiation, not the bound.

```ds
type Flags<T: { a: number }> = { [K in keyof T]: boolean };

type Actual = Flags<type { a: number; b: string }>;

const ok: Actual = { a: true, b: false };
ok satisfies Actual;
```

### key remapping supports indexed reads

Remapped keys still project their source field types.

```ds
type Prefix<T> = {
    [K in keyof T as `get${Capitalize<K & string>}`]: T[K];
};

type Accessors = Prefix<type { name: string; age: int32 }>;
type Values = Accessors["getName" | "getAge"];

const name: Values = "Ada";
const age: Values = 42;
```

### key remapping rejects missing keys

Only remapped keys exist.

```ds
type Prefix<T> = {
    [K in keyof T as `get${Capitalize<K & string>}`]: T[K];
};

type Accessors = Prefix<type { name: string }>;
type Missing = Accessors["name"];
```

- contains: does not exist

## optional

### optional mapped reads include undefined

`?` adds `undefined` to every projection.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

type Values = Optional<type { name: string }>["name"];

const missing: Values = undefined;
```

### optional mapped reads reject unrelated values

Optionality adds nothing else.

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

type Values = Optional<type { name: string }>["name"];

const bad: Values = 1;
```

- contains: not assignable

## imports

### mapped aliases survive imports

Mapped types keep working across modules.

```ds:library.ds
export type Optional<T> = { [K in keyof T]?: T[K] };
export type Box<T> = Optional<T>;
```

```ds:main.ds
import { Box } from "./library.ds";

type Name = Box<type { name: string }>["name"];

const ok: Name = "Ada";
const missing: Name = undefined;
```
