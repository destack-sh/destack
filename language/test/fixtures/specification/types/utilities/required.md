# Required

`Required` is a standard utility type.

## properties

### required removes optionality

Every field becomes mandatory.

```ds
interface Person {
    name?: string;
    age?: number;
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: "Ada", age: 42 };
ok satisfies FullPerson;
```

### required rejects missing fields

Nothing stays optional.

```ds
interface Person {
    name?: string;
    age?: number;
}

type FullPerson = Required<Person>;

const bad: FullPerson = { name: "Ada" };
```

- contains: not assignable

### required keeps undefined in property types

Explicit `undefined` in the type is not optionality.

```ds
interface Person {
    name?: string | undefined;
}

type FullPerson = Required<Person>;

const ok: FullPerson = { name: undefined };
ok satisfies FullPerson;
```

### required preserves readonly fields

Modifiers other than `?` survive.

```ds
interface Person {
    readonly name?: string;
}

type FullPerson = Required<Person>;

const person: FullPerson = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
