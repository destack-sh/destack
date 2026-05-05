# Omit

`Omit` is a standard TypeScript utility type.

## cases

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

### omit preserves readonly properties

> Properties that remain after omission keep readonly modifiers.

```ds libs=es5
interface Person {
    readonly name: string
    age: number
}

type NameOnly = Omit<Person, "age">;

const person: NameOnly = { name: "Ada" };
person.name = "Grace";
```

- contains: read-only
