# Template Assignability

## assignability

### template literal type assigns to broader template

Narrow template literal types assign to broader ones.

```ds
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare let tight: Tight;
let ok: Loose = tight;
```

### template literal type rejects union assignment to narrower template

Union template types are not assignable to a narrower branch.

```ds
type Combo = `foo-${string}` | `bar-${string}`;
type FooOnly = `foo-${string}`;

declare let combo: Combo;
let bad: FooOnly = combo;
```

- contains: not assignable to type fooonly

### template literal type rejects assignment to narrower template

Broader template literal types do not assign to narrower ones.

```ds
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare let loose: Loose;
let bad: Tight = loose;
```

- contains: not assignable

### template literal type accepts generic spans

Generic spans assign to the string supertype.

```ds
type AnyString = `${string}`;
type Tagged = `tag-${string}`;

declare let tagged: Tagged;
let ok: AnyString = tagged;
```

### template literal numeric spans assign to string spans

Numeric spans are assignable to string spans.

```ds
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare let numeric: NumericId;
let ok: StringId = numeric;
```

### template literal string spans reject numeric spans

String spans do not narrow to numeric spans.

```ds
type NumericId = `id-${number}`;
type StringId = `id-${string}`;

declare let value: StringId;
let bad: NumericId = value;
```

- contains: not assignable to type numericid

### template literal type assigns to string

Template literal types assign to string.

```ds
type Tagged = `tag-${string}`;

declare let tagged: Tagged;
let ok: string = tagged;
```

### string does not assign to template literal type

Strings do not assign to narrower template literal types.

```ds
type Tagged = `tag-${string}`;

declare let value: string;
let bad: Tagged = value;
```

- contains: not assignable

### template literal type rejects boolean spans

Boolean spans are not assignable to numeric spans.

```ds
type BoolSpan = `flag-${boolean}`;
type NumSpan = `flag-${number}`;

declare let value: BoolSpan;
let bad: NumSpan = value;
```

- contains: not assignable
