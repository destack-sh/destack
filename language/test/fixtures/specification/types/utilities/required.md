# Required

`Required` is a standard TypeScript utility type.

## cases

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

### required keeps undefined in property types

> `Required` removes optionality without removing explicit undefined.

```ds libs=es5
interface Person {
    name?: string | undefined
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: undefined };
ok satisfies FullPerson;
```

### required preserves readonly fields

> `Required` keeps readonly modifiers from the source type.

```ds libs=es5
interface Person {
    readonly name?: string
}

type FullPerson = Required<Person>;

const person: FullPerson = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
