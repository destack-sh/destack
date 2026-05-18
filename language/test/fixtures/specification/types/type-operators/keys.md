# Keys

`keyof` and static `in` work over the apparent keys of a type.

## keyof

### keyof builds literal key unions

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

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left | Right);

const ok: Keys = "shared";
```

### keyof unions reject missing keys

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left | Right);

const bad: Keys = "left";
```

- contains: not assignable

### keyof intersections include all keys

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type Keys = keyof (Left & Right);

const shared: Keys = "shared";
const left: Keys = "left";
const right: Keys = "right";
```

### keyof uses instantiated keys

```ds
type Keys<T: { a: number }> = keyof T;
type Actual = Keys<type { a: number; b: string }>;

const key: Actual = "b";
key satisfies "a" | "b";
```

## index signatures

### string indexes contribute string and number keys

```ds
interface Bag {
    [key: string]: number;
}

type Keys = keyof Bag;

const okString: Keys = "a";
const okNumber: Keys = 1;
```

### string indexes reject boolean keys

```ds
interface Bag {
    [key: string]: number;
}

type Keys = keyof Bag;

const bad: Keys = true;
```

- contains: not assignable

### number indexes contribute number keys

```ds
interface NumberBag {
    [key: number]: string;
}

type Keys = keyof NumberBag;

const ok: Keys = 1;
```

### number indexes reject string keys

```ds
interface NumberBag {
    [key: number]: string;
}

type Keys = keyof NumberBag;

const bad: Keys = "name";
```

- contains: not assignable

## in

### in returns true for existing keys

```ds
interface Person {
    name: string;
    age: number;
}

type HasName = "name" in Person;

const ok: HasName = true;
```

### in returns false for missing keys

```ds
interface Person {
    name: string;
    age: number;
}

type HasTitle = "title" in Person;

const ok: HasTitle = false;
```

### in uses shared union keys

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasLeft = "left" in (Left | Right);
type HasShared = "shared" in (Left | Right);

const left: HasLeft = false;
const shared: HasShared = true;
```

### in uses intersection keys

```ds
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasLeft = "left" in (Left & Right);

const ok: HasLeft = true;
```
