# Partial

`Partial` is a standard utility type.

### partial allows missing fields

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

### partial rejects extra fields

```ds
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const bad: OptionalPerson = { name: "Ada", extra: true };
```

- contains: excess property 'extra'

### partial rejects incompatible field types

```ds
interface Person {
    name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const bad: OptionalPerson = { name: "Ada", age: "no" };
```

- contains: not assignable

### partial preserves readonly fields

```ds
interface Person {
    readonly name: string
    age: number
}

type OptionalPerson = Partial<Person>;

const person: OptionalPerson = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
