# Type Operators

## keyof builds literal key unions

> `keyof` produces a union of literal keys.

```ds
interface Person {
    name: string
    age: number
}

type Keys = keyof Person;

const name: Keys = "name";
const age: Keys = "age";
const bad: Keys = "title";
```

- contains: not assignable

## indexed access returns property types

> Indexed access resolves to the property type.

```ds
interface Person {
    name: string
}

type Name = Person["name"];

const ok: Name = "Ada";
const bad: Name = 42;
```

- contains: not assignable
