# Tuple Types

Tuple types use parenthesized syntax in `.ds`: `(T, U)`.
Bracket tuple syntax remains accepted in `.ts` files and is not the `.ds` tuple form.

## positional typing

### parenthesized tuple positions carry declared element types

> Tuple positions carry their declared element types.

```ds
const pair: (int32, string) = (1, "hello");
```

### parenthesized tuple arity is enforced on assignment

> Tuple assignments must satisfy fixed positional arity.

```ds
const pair: (int32, string) = (1);
```

- contains: not assignable

### parenthesized tuple positional element types are enforced

> Tuple assignments must satisfy each positional element type.

```ds
const pair: (int32, string) = ("one", 2);
```

- contains: not assignable

### bracket tuple syntax is rejected in ds sources

> Bracket tuple syntax is not the `.ds` tuple form.

```ds
const pair: [int32, string] = [1, "hello"];
```

- contains: tuple

### nested tuples preserve nested element types

> Nested tuples preserve nested element types.

```ds
const value: (int32, (string, boolean)) = (1, ("hello", true));
```

## tuple and array assignability

### parenthesized tuples are assignable to dynamic arrays when elements are compatible

> Tuples are assignable to dynamic arrays when element types are compatible.

```ds
const pair: (int32, int32) = (1, 2);
const values: int32[] = pair;
```

### tuples reject assignment to incompatible dynamic arrays

> Tuples reject assignment to dynamic arrays with incompatible element types.

```ds
const pair: (int32, int32) = (1, 2);
const values: string[] = pair;
```

- contains: not assignable

### dynamic arrays are not assignable to fixed tuples

> Dynamic arrays are not assignable to fixed tuples.

```ds
const values: int32[] = [1, 2];
const pair: (int32, int32) = values;
```

- contains: not assignable
