# Utility

## standard helpers

### exclude distributes over unions

> Conditional types distribute over union types.

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

### exclude rejects removed members

> Excluded members are not assignable.

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

### exclude with never yields never

> Excluding from never yields never.

```ds libs=es5
type NeverLetters = Exclude<never, "b">;

let bad: NeverLetters = "b";
```

- contains: not assignable

### pick preserves optional properties

> `Pick` keeps optionality from the source type.

```ds libs=es5
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;

const ok: AgeOnly = {};
ok satisfies AgeOnly;
const ok2 = { age: 42 };
ok2 satisfies AgeOnly;
```

### pick rejects extra fields on optional picks

> Picked types reject extra fields on literals.

```ds libs=es5
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;

const bad: AgeOnly = { name: "Ada" };
```

- contains: excess property 'name'

### pick accepts required fields

> Picked types accept object literals with only picked keys.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const ok: NameOnly = { name: "Ada" };
ok satisfies NameOnly;
```

### pick accepts union keys

> Picked types accept multiple keys.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameAge = Pick<Person, "name" | "age">;

const ok: NameAge = { name: "Ada", age: 42 };
ok satisfies NameAge;
```

### pick merges shared union key types

> Picked unions merge shared key types.

```ds libs=es5
type Mixed = { value: string } | { value: int32 };
type Picked = Pick<Mixed, "value">;

const ok: Picked = { value: "Ada" };
const ok2: Picked = { value: 42 };
```

### pick rejects non member union values

> Picked unions reject values outside the merged type.

```ds libs=es5
type Mixed = { value: string } | { value: int32 };
type Picked = Pick<Mixed, "value">;

const bad: Picked = { value: true };
```

- contains: not assignable

### pick rejects missing union keys

> Picked unions require all selected keys.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameAge = Pick<Person, "name" | "age">;

const bad: NameAge = { name: "Ada" };
```

- contains: not assignable

### pick rejects extra fields

> Picked types reject extra fields on literals.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const bad: NameOnly = { name: "Ada", extra: true };
```

- contains: excess property 'extra'

### pick rejects extra fields with required keys

> Picked types reject extra fields on literals.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const bad: NameOnly = { name: "Ada", age: 42 };
```

- contains: excess property 'age'

### pick rejects missing required fields

> Picked types still require their fields.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const bad: NameOnly = {};
```

- contains: not assignable

### pick rejects unknown keys

> Pick keys must be part of the source type.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name" | "missing">;
```

- contains: not assignable

### omit removes selected keys

> `Omit` removes keys before assignment checks.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;

const ok: WithoutAge = { name: "Ada" };
ok satisfies WithoutAge;
```

### omit rejects removed keys

> Omitted keys are not assignable on literals.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;

const bad: WithoutAge = { name: "Ada", age: 42 };
```

- contains: excess property 'age'

### omit with union keys removes all

> Omit removes every listed key.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type WithoutAll = Omit<Person, "name" | "age">;

const ok: WithoutAll = {};
```

### omit with union keys rejects removed fields

> Omitted keys are rejected.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type WithoutAll = Omit<Person, "name" | "age">;

const bad: WithoutAll = { name: "Ada" };
```

- contains: excess property 'name'

### omit ignores unknown keys

> Omit does not require keys to exist and ignores missing keys.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "missing">;

const ok: WithoutAge = { name: "Ada", age: 42 };
ok satisfies Person;
```

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

### partial allows missing fields

> `Partial` makes every property optional.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const ok: OptionalPerson = {};
ok satisfies OptionalPerson;
const ok2 = { name: "Ada" };
ok2 satisfies OptionalPerson;
```

### partial rejects extra fields

> Extra fields are rejected on literals.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const bad: OptionalPerson = { name: "Ada", extra: true };
```

- contains: excess property 'extra'

### partial rejects incompatible field types

> Optional fields must still satisfy their declared types.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const bad: OptionalPerson = { name: "Ada", age: "no" };
```

- contains: not assignable

### required removes optionality

> `Required` removes optional modifiers.

```ds libs=es5
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: "Ada", age: 42 };
ok satisfies FullPerson;
```

### required rejects missing fields

> Missing fields are rejected.

```ds libs=es5
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const bad: FullPerson = { name: "Ada" };
```

- contains: not assignable

### readonly keeps field types

> `Readonly` preserves the field types.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

### readonly preserves optional fields

> `Readonly` keeps optional fields optional.

```ds libs=es5
interface Person {
    name?: string
}

type Frozen = Readonly<Person>;

const ok: Frozen = {};
ok satisfies Frozen;
```

### readonly rejects mutable assignment

> Readonly fields are not assignable to mutable fields.

```ds libs=es5
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const frozen: Frozen = { name: "Ada", age: 42 };
const bad: Person = frozen;
```

- contains: not assignable

### nonnullable removes nullish

> `NonNullable` strips nullish members.

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

### nonnullable rejects nullish values

> Nullish values are rejected.

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

### nonnullable with never yields never

> NonNullable preserves never.

```ds libs=es5
type NeverValue = NonNullable<never>;

let bad: NeverValue = "no";
```

- contains: not assignable

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

### extract keeps matching members

> `Extract` keeps the overlapping members.

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const ok: OnlyAorB = "a";
ok satisfies OnlyAorB;
const ok2: OnlyAorB = "b";
ok2 satisfies OnlyAorB;
```

### extract rejects non members

> Non matching members are rejected.

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const bad: OnlyAorB = "c";
```

- contains: not assignable

### extract with never yields never

> Extracting from never yields never.

```ds libs=es5
type NeverLetters = Extract<never, "a">;

let bad: NeverLetters = "a";
```

- contains: not assignable
