# Union Types

Union type assignability.

## unions

### string or number

> Union types accept any of their member types.

```ds
let x: string | number = "hello"
x = 42
```

### assign to union member

> Any union member can be assigned to the union.

```ds
const x: string | number = "hello"
```

## Union Assignability

### union element assignable to union

> A value of a member type is assignable to the union.

```ds
const x: string = "hello"
const y: string | number = x
```

## Union Normalization

### union flattening through aliases

> Nested aliases flatten into a single union.

```ds
type A = { a: number } | { b: string };
type B = A | { c: boolean };
const value: B = { c: true };
value satisfies { a: number } | { b: string } | { c: boolean };
```

### union removes never

> Never members are removed from unions.

```ds
type A = never | string;
const value: A = "hello";
value satisfies string;
```

