# Keys

`keyof` and conditional key membership work over the apparent keys of a type.

## keyof

### keyof builds literal key unions

`keyof` enumerates the declared keys as literals.

```ds
interface Person {
    name: string;
    age: number;
}

type Keys = keyof Person;

const name: Keys = "name";
const age: Keys = "age";
```

### keyof rejects unknown keys

The union is closed.

```ds
interface Person {
    name: string;
    age: number;
}

type Keys = keyof Person;

const bad: Keys = "title";
```

- contains: not assignable

### keyof unions keep shared keys

A union only guarantees its common keys.

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left | Right);

const ok: Keys = "shared";
```

### keyof unions reject missing keys

Arm-specific keys are not shared.

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left | Right);

const bad: Keys = "left";
```

- contains: not assignable

### keyof intersections include all keys

An intersection guarantees every key.

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left & Right);

const shared: Keys = "shared";
const left: Keys = "left";
const right: Keys = "right";
```

### keyof uses instantiated keys

The operator sees the actual instantiation, not the bound.

```ds
type Keys<T: { a: number }> = keyof T;
type Actual = Keys<{ a: number; b: string }>;

const key: Actual = "b";
key satisfies "a" | "b";
```

## index signatures

### string indexes contribute string and usize keys

A string index admits string keys and `usize` keys.

```ds
interface Bag {
    [key: string]: int32;
}

type Keys = keyof Bag;

const okString: Keys = "a";
const okIndex: Keys = 1;
```

### string indexes reject boolean keys

Only key types participate.

```ds
interface Bag {
    [key: string]: int32;
}

type Keys = keyof Bag;

const bad: Keys = true;
```

- contains: not assignable

### usize indexes contribute usize keys

A usize index admits usize keys.

```ds
interface SlotBag {
    [key: usize]: string;
}

type Keys = keyof SlotBag;

const ok: Keys = 1;
```

### usize indexes reject string keys

Usize indexes do not admit strings.

```ds
interface SlotBag {
    [key: usize]: string;
}

type Keys = keyof SlotBag;

const bad: Keys = "name";
```

- contains: not assignable

## key membership

### conditionals answer existing keys

Key membership at the type level is spelled through `keyof` and a conditional, as in TypeScript.

```ds
interface Person {
    name: string;
    age: number;
}

type HasName = "name" extends keyof Person ? true : false;

const ok: HasName = true;
```

### conditionals answer missing keys

Missing keys answer `false`.

```ds
interface Person {
    name: string;
    age: number;
}

type HasTitle = "title" extends keyof Person ? true : false;

const ok: HasTitle = false;
```

### membership uses shared union keys

Union membership follows the shared keys.

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasLeft = "left" extends keyof (Left | Right) ? true : false;
type HasShared = "shared" extends keyof (Left | Right) ? true : false;

const left: HasLeft = false;
const shared: HasShared = true;
```

### membership uses intersection keys

Intersection membership includes every key.

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasLeft = "left" extends keyof (Left & Right) ? true : false;

const ok: HasLeft = true;
```
