# Record

`Record` is a standard utility type.

## objects

### record builds required properties

```ds
type Flags = Record<"a" | "b", boolean>;

const ok: Flags = { a: true, b: false };
ok satisfies Flags;
```

### record rejects invalid key types

```ds
type Bad = Record<type { name: string }, boolean>;
```

- contains: not assignable

### record requires all keys

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true };
```

- contains: not assignable

### record supports numeric keys

```ds
type NumericFlags = Record<1 | 2, string>;

const ok: NumericFlags = { 1: "one", 2: "two" };
ok satisfies NumericFlags;
```

### record supports string and number keys

```ds
type Lookup = Record<string | number, string>;

const ok: Lookup = { name: "Ada", 1: "one" };
ok satisfies Lookup;
```

### record supports symbol keys

```ds
declare const key: unique symbol;

type Flags = Record<typeof key, boolean>;

const ok: Flags = { [key]: true };
ok[key] satisfies boolean;
```

### record rejects missing numeric keys

```ds
type NumericFlags = Record<1 | 2, string>;

const bad: NumericFlags = { 1: "one" };
```

- contains: not assignable

### record rejects extra keys

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true, b: false, c: true };
```

- contains: excess property 'c'

### record with never yields empty object

```ds
type Empty = Record<never, boolean>;

const ok: Empty = {};
```

### record with never rejects extra fields

```ds
type Empty = Record<never, boolean>;

const bad: Empty = { value: true };
```

- contains: excess property 'value'
