# Record

`Record` is a standard utility type.

## objects

### record builds required properties

Each key becomes a required property.

```ds
type Flags = Record<"a" | "b", boolean>;

const ok: Flags = { a: true, b: false };
ok satisfies Flags;
```

### record rejects invalid key types

Keys must be key-typed.

```ds
type Bad = Record<type { name: string }, boolean>;
```

- contains: not assignable

### record requires all keys

Finite records are exhaustive.

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true };
```

- contains: not assignable

### record supports numeric keys

Numeric literal keys work.

```ds
type NumericFlags = Record<1 | 2, string>;

const ok: NumericFlags = { 1: "one", 2: "two" };
ok satisfies NumericFlags;
```

### record with string keys requires writable index access

A broad string `Record` cannot be satisfied by a finite object shape.

```ds
type Lookup = Record<string, string>;

const value = { name: "Ada" };
const lookup: Lookup = value;
```

- contains: not assignable

### record supports symbol keys

Unique symbol keys work.

```ds
declare const key: unique symbol;

type Flags = Record<typeof key, boolean>;

const ok: Flags = { [key]: true };
ok[key] satisfies boolean;
```

### record rejects missing numeric keys

Numeric records are exhaustive too.

```ds
type NumericFlags = Record<1 | 2, string>;

const bad: NumericFlags = { 1: "one" };
```

- contains: not assignable

### record rejects extra keys

The shape is closed.

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true, b: false, c: true };
```

- contains: excess property 'c'

### record with never yields empty object

No keys, no properties.

```ds
type Empty = Record<never, boolean>;

const ok: Empty = {};
```

### record with never rejects extra fields

Empty stays empty.

```ds
type Empty = Record<never, boolean>;

const bad: Empty = { value: true };
```

- contains: excess property 'value'
