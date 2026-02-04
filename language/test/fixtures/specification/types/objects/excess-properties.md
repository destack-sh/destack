# Excess Property Checks

## unions

### union rejects extra fields

> Object literals with fields not present in any union member are rejected.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named | Aged = { name: "Ada", extra: true };
```

- contains: excess property

## intersections

### intersection requires all fields

> Object literals must satisfy every intersection member.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named & Aged = { name: "Ada" };
```

- contains: not assignable

### intersection accepts combined fields

> Object literals satisfy intersections when all fields are present.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named & Aged = { name: "Ada", age: 42 };
```
