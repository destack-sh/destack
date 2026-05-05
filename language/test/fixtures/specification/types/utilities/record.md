# Record

`Record` is a standard TypeScript utility type.

## cases

### record builds required properties

> `Record` produces required fields for each key.

```ds libs=es5
type Flags = Record<"a" | "b", boolean>;

const ok: Flags = { a: true, b: false };
ok satisfies Flags;
```

### record rejects invalid key types

> Record keys must be string, number, or symbol.

```ds libs=es5
type Bad = Record<{ name: string }, boolean>;
```

- contains: not assignable

### record requires all keys

> Missing keys are rejected.

```ds libs=es5
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true };
```

- contains: not assignable

### record supports numeric keys

> Numeric literal keys are accepted.

```ds libs=es5
type NumericFlags = Record<1 | 2, string>;

const ok: NumericFlags = { 1: "one", 2: "two" };
ok satisfies NumericFlags;
```

### record supports symbol keys

> Symbol keys are accepted.

```ts libs=es5
declare const key: unique symbol;

type Flags = Record<typeof key, boolean>;

const ok: Flags = { [key]: true };
ok[key] satisfies boolean;
```

### record rejects missing numeric keys

> Missing numeric keys are rejected.

```ds libs=es5
type NumericFlags = Record<1 | 2, string>;

const bad: NumericFlags = { 1: "one" };
```

- contains: not assignable

### record rejects extra keys

> Extra keys are rejected.

```ds libs=es5
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true, b: false, c: true };
```

- contains: excess property 'c'

### record with never yields empty object

> Record over never produces an empty object type.

```ds libs=es5
type Empty = Record<never, boolean>;

const ok: Empty = {};
```

### record with never rejects extra fields

> Record over never rejects extra properties.

```ds libs=es5
type Empty = Record<never, boolean>;

const bad: Empty = { value: true };
```

- contains: excess property 'value'
