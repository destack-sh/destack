# Utility Types

## exclude distributes over unions

> Conditional types distribute over union types.

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

## exclude rejects removed members

> Excluded members are not assignable.

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

## pick preserves optional properties

> `Pick` keeps optionality from the source type.

```ds
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

## pick rejects extra fields on optional picks

> Picked types reject extra fields on literals.

```ds
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;

const bad: AgeOnly = { name: "Ada" };
```

- contains: excess property

## pick rejects extra fields

> Picked types reject extra fields on literals.

```ds
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const ok: NameOnly = { name: "Ada" };
ok satisfies NameOnly;
```

## pick rejects extra fields with required keys

> Picked types reject extra fields on literals.

```ds
interface Person {
    name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const bad: NameOnly = { name: "Ada", age: 42 };
```

- contains: excess property

## omit removes selected keys

> `Omit` removes keys before assignment checks.

```ds
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;

const ok: WithoutAge = { name: "Ada" };
ok satisfies WithoutAge;
```

## omit rejects removed keys

> Omitted keys are not assignable on literals.

```ds
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;

const bad: WithoutAge = { name: "Ada", age: 42 };
```

- contains: excess property

## record builds required properties

> `Record` produces required fields for each key.

```ds
type Flags = Record<"a" | "b", boolean>;

const ok: Flags = { a: true, b: false };
ok satisfies Flags;
```

## record requires all keys

> Missing keys are rejected.

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true };
```

- contains: not assignable

## record rejects extra keys

> Extra keys are rejected.

```ds
type Flags = Record<"a" | "b", boolean>;

const bad: Flags = { a: true, b: false, c: true };
```

- contains: excess property

## partial allows missing fields

> `Partial` makes every property optional.

```ds
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

## partial rejects extra fields

> Extra fields are rejected on literals.

```ds
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const bad: OptionalPerson = { name: "Ada", extra: true };
```

- contains: excess property

## required removes optionality

> `Required` removes optional modifiers.

```ds
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: "Ada", age: 42 };
ok satisfies FullPerson;
```

## required rejects missing fields

> Missing fields are rejected.

```ds
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const bad: FullPerson = { name: "Ada" };
```

- contains: not assignable

## readonly keeps field types

> `Readonly` preserves the field types.

```ds
interface Person {
    name: string
    age: number
}

type Frozen = Readonly<Person>;

const ok: Frozen = { name: "Ada", age: 42 };
ok satisfies Frozen;
```

## nonnullable removes nullish

> `NonNullable` strips nullish members.

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

## nonnullable rejects nullish values

> Nullish values are rejected.

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

## extract keeps matching members

> `Extract` keeps the overlapping members.

```ds
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const ok: OnlyAorB = "a";
ok satisfies OnlyAorB;
const ok2: OnlyAorB = "b";
ok2 satisfies OnlyAorB;
```

## extract rejects non members

> Non matching members are rejected.

```ds
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const bad: OnlyAorB = "c";
```

- contains: not assignable
