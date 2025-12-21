# Union Types

Tests for union type assignability.

## Basic Unions

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
