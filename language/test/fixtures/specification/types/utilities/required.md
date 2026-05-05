# Required

`Required` is a standard utility type.

### required removes optionality

```ds
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: "Ada", age: 42 };
ok satisfies FullPerson;
```

### required rejects missing fields

```ds
interface Person {
    name?: string
    age?: number
}

type FullPerson = Required<Person>;

const bad: FullPerson = { name: "Ada" };
```

- contains: not assignable

### required keeps undefined in property types

```ds
interface Person {
    name?: string | undefined
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: undefined };
ok satisfies FullPerson;
```

### required preserves readonly fields

```ds
interface Person {
    readonly name?: string
}

type FullPerson = Required<Person>;

const person: FullPerson = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
