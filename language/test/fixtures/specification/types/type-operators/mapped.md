# Mapped Types

Mapped types iterate over static key unions.

## keys

### mapped types use instantiated keys

```ds
type Flags<T: { a: number }> = { [K in keyof T]: boolean };

type Actual = Flags<{ a: number; b: string }>;

const ok: Actual = { a: true, b: false };
ok satisfies Actual;
```

### key remapping supports indexed reads

```ds
type Prefix<T> = {
    [K in keyof T as `get${Capitalize<K & string>}`]: T[K];
};

type Accessors = Prefix<{ name: string; age: int32 }>;
type Values = Accessors["getName" | "getAge"];

const name: Values = "Ada";
const age: Values = 42;
```

### key remapping rejects missing keys

```ds
type Prefix<T> = {
    [K in keyof T as `get${Capitalize<K & string>}`]: T[K];
};

type Accessors = Prefix<{ name: string }>;
type Missing = Accessors["name"];
```

- contains: does not exist

## optional

### optional mapped reads include undefined

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

type Values = Optional<{ name: string }>["name"];

const missing: Values = undefined;
```

### optional mapped reads reject unrelated values

```ds
type Optional<T> = { [K in keyof T]?: T[K] };

type Values = Optional<{ name: string }>["name"];

const bad: Values = 1;
```

- contains: not assignable

## imports

### mapped aliases survive imports

```ds:library.ds
export type Optional<T> = { [K in keyof T]?: T[K] };
export type Box<T> = Optional<T>;
```

```ds:main.ds
import { Box } from "./library.ds";

type Name = Box<{ name: string }>["name"];

const ok: Name = "Ada";
const missing: Name = undefined;
```
