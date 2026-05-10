# Omit

`Omit` is a standard utility type.

## objects

### omit removes selected keys

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

```ds
interface Person {
    name: string;
    age: number;
}

type WithoutAll = Omit<Person, "name" | "age">;

const ok: WithoutAll = {};
```

### omit with union keys rejects removed fields

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
