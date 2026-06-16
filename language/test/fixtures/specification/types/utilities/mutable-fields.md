# MutableFields

`MutableFields<T>` is the mapped utility for shallow mutable fields.

## properties

### mutable fields removes readonly field modifiers

Readonly fields become writable.

```ds
interface Person {
    readonly name: string;
    age: number;
}

type MutablePerson = MutableFields<Person>;

let person: MutablePerson = { name: "Ada", age: 42 };
person.name = "Grace";
person.name satisfies string;
```

### mutable fields preserves optional fields

Optionality survives.

```ds
interface Person {
    readonly name?: string;
}

type MutablePerson = MutableFields<Person>;

const empty: MutablePerson = {};
const named: MutablePerson = { name: "Ada" };
empty satisfies MutablePerson;
named satisfies MutablePerson;
```

### mutable fields keeps nested readonly views

The utility only changes the immediate field modifier.

```ds
interface Person {
    readonly profile: readonly {
        name: string;
    };
}

type MutablePerson = MutableFields<Person>;

let person: MutablePerson = { profile: { name: "Ada" } };
person.profile.name = "Grace";
```

- contains: readonly
