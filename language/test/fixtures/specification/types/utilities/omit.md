# Omit

`Omit` is a standard utility type.

## objects

### omit removes selected keys

Omitted keys leave the shape.

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAge = Omit<Person, "age">;

const ok: WithoutAge = { name: "Ada" };
ok satisfies WithoutAge;
```

### omit rejects removed keys

Removed fields are gone for writers too.

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAge = Omit<Person, "age">;

const bad: WithoutAge = { name: "Ada", age: 42 };
```

- contains: excess property 'age'

### omit with union keys removes all

A key union omits several fields.

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAll = Omit<Person, "name" | "age">;

const ok: WithoutAll = {};
```

### omit with union keys rejects removed fields

Every omitted field is gone.

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAll = Omit<Person, "name" | "age">;

const bad: WithoutAll = { name: "Ada" };
```

- contains: excess property 'name'

### omit ignores unknown keys

Omitting nothing changes nothing.

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAge = Omit<Person, "missing">;

const ok: WithoutAge = { name: "Ada", age: 42 };
ok satisfies Person;
```

### omit preserves readonly properties

Modifiers survive on the keys that remain.

```ds
interface Person {
    readonly name: string;
    age: number;
}

type NameOnly = Omit<Person, "age">;

const person: NameOnly = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
