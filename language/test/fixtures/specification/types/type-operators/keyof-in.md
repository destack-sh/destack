# Keyof and In

## keys

### keyof builds literal key unions

> `keyof` produces a union of literal keys.

```ds
interface Person {
    name: string
    age: number
}

type Keys = keyof Person;

const name: Keys = "name";
const age: Keys = "age";
```

### keyof rejects unknown keys

> Keys outside the union are rejected.

```ds
interface Person {
    name: string
    age: number
}

type Keys = keyof Person;

const bad: Keys = "title";
```

- type 42 is not assignable to type Name

### keyof union keeps shared keys

> `keyof` on unions keeps only the shared keys.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type Keys = keyof (Left | Right);

const ok: Keys = "shared";
```

### keyof union rejects non shared keys

> Union keys exclude members not present on every branch.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type Keys = keyof (Left | Right);

const bad: Keys = "left";
```

- type 42 is not assignable to type Name

### keyof intersection includes all keys

> `keyof` on intersections includes every key.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type Keys = keyof (Left & Right);

const ok: Keys = "shared";
const ok2: Keys = "left";
const ok3: Keys = "right";
```

### keyof intersection rejects unknown keys

> Intersection keys still reject unrelated names.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type Keys = keyof (Left & Right);

const bad: Keys = "missing";
```

- type 42 is not assignable to type Name

### keyof includes string index signatures

> String index signatures contribute string and number keys.

```ds
interface Bag {
    [key: string]: number
}

type Keys = keyof Bag;

const okString: Keys = "a";
const okNumber: Keys = 1;
```

### keyof rejects non string or number keys

> String index signatures do not include boolean keys.

```ds
interface Bag {
    [key: string]: number
}

type Keys = keyof Bag;

const bad: Keys = true;
```

- type 42 is not assignable to type Name

### keyof includes number index signatures

> Number index signatures contribute numeric keys.

```ds
interface NumberBag {
    [key: number]: string
}

type Keys = keyof NumberBag;

const ok: Keys = 1;
```

### keyof rejects non numeric keys for number indexes

> Number index signatures do not include string keys.

```ds
interface NumberBag {
    [key: number]: string
}

type Keys = keyof NumberBag;

const bad: Keys = "name";
```

- type 42 is not assignable to type Name

### keyof union keeps shared keys with index signatures

> Union keys intersect with index signatures.

```ds
interface TextBag {
    [key: string]: number
}

interface FlagBag {
    flag: boolean
}

type Keys = keyof (TextBag | FlagBag);

const ok: Keys = "flag";
```

### keyof union rejects missing index keys

> Index signature keys are dropped when not shared.

```ds
interface TextBag {
    [key: string]: number
}

interface FlagBag {
    flag: boolean
}

type Keys = keyof (TextBag | FlagBag);

const bad: Keys = 1;
```

- contains: not assignable

### keyof union keeps numeric keys from numeric index signatures

> Numeric index signatures contribute shared numeric keys.

```ds
interface StringIndex {
    [key: string]: number
}

interface NumberIndex {
    [key: number]: string
}

type Keys = keyof (StringIndex | NumberIndex);

const ok: Keys = 1;
```

### keyof union rejects string keys without string index

> String keys are rejected when not present on all members.

```ds
interface StringIndex {
    [key: string]: number
}

interface NumberIndex {
    [key: number]: string
}

type Keys = keyof (StringIndex | NumberIndex);

const bad: Keys = "name";
```

- contains: not assignable

### keyof union with null yields never

> Non object union members remove keys.

```ds
type Keys = keyof ({ name: string } | null);

const bad: Keys = "name";
```

- contains: not assignable

### in returns true for existing keys

> `in` checks whether a key type is assignable to `keyof`.

```ds
interface Person {
    name: string
    age: number
}

type HasName = "name" in Person;

const ok: HasName = true;
```

### in rejects false for existing keys

> Existing keys do not produce `false`.

```ds
interface Person {
    name: string
    age: number
}

type HasName = "name" in Person;

const bad: HasName = false;
```

- contains: not assignable

### in returns false for missing keys

> Missing keys result in `false`.

```ds
interface Person {
    name: string
    age: number
}

type HasTitle = "title" in Person;

const ok: HasTitle = false;
```

### in rejects true for missing keys

> Missing keys do not produce `true`.

```ds
interface Person {
    name: string
    age: number
}

type HasTitle = "title" in Person;

const bad: HasTitle = true;
```

- contains: not assignable

### in returns false for missing union keys

> Union keys must exist on every branch.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type HasLeft = "left" in (Left | Right);

const ok: HasLeft = false;
```

### in rejects true for missing union keys

> Union keys reject `true` when the key is missing.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type HasLeft = "left" in (Left | Right);

const bad: HasLeft = true;
```

- contains: not assignable

### in returns true for intersection keys

> Intersection keys include every key.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type HasLeft = "left" in (Left & Right);

const ok: HasLeft = true;
```

### in rejects false for intersection keys

> Intersection keys reject `false`.

```ds
interface Left {
    shared: string
    left: int32
}

interface Right {
    shared: string
    right: int32
}

type HasLeft = "left" in (Left & Right);

const bad: HasLeft = false;
```

- contains: not assignable

### indexed access returns property types

> Indexed access resolves to the property type.

```ds
interface Person {
    name: string
}

type Name = Person["name"];

const ok: Name = "Ada";
```

### indexed access rejects incompatible values

> Indexed access must satisfy the property type.

```ds
interface Person {
    name: string
}

type Name = Person["name"];

const bad: Name = 42;
```

- contains: not assignable
