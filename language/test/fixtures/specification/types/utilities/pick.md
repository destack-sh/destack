# Pick

`Pick` is a standard TypeScript utility type.

## cases

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

### pick preserves readonly properties

> Picked properties keep readonly modifiers.

```ds libs=es5
interface Person {
    readonly name: string
    age: number
}

type NameOnly = Pick<Person, "name">;

const person: NameOnly = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
