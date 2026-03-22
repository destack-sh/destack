# Tuple Types

Tuple type fixtures lock DS tuple semantics.
Tuple types support both parenthesized and bracket syntax.
These are equivalent tuple type constructors: `(T, U)` and `[T, U]`.

## positional typing

### parenthesized tuple positions carry declared element types

> Tuple positions carry their declared element types.

```ds
const pair: (int32, string) = (1, "hello");
```

### bracket tuple positions carry declared element types

> Bracket tuple positions carry their declared element types.

```ds
const pair: [int32, string] = [1, "hello"];
```

### tuple syntaxes are mutually assignable

> Parenthesized and bracket tuple annotations represent the same tuple shape.

```ds
const left: (int32, string) = [1, "hello"];
const right: [int32, string] = (1, "hello");
```

### parenthesized tuple arity is enforced on assignment

> Tuple assignments must satisfy fixed positional arity.

```ds
const pair: (int32, string) = (1);
```

- type int32[] is not assignable to type (int32, int32)

### bracket tuple arity is enforced on assignment

> Bracket tuple assignments must satisfy fixed positional arity.

```ds
const pair: [int32, string] = [1];
```

- type int32[] is not assignable to type (int32, int32)

### parenthesized tuple positional element types are enforced

> Tuple assignments must satisfy each positional element type.

```ds
const pair: (int32, string) = ("one", 2);
```

- not assignable

### bracket tuple positional element types are enforced

> Bracket tuple assignments must satisfy each positional element type.

```ds
const pair: [int32, string] = ["one", 2];
```

- not assignable

### nested tuples preserve nested element types across both syntaxes

> Nested tuples preserve nested element types across both syntax forms.

```ds
const left: (int32, [string, boolean]) = (1, ["hello", true]);
const right: [int32, (string, boolean)] = [1, ("hello", true)];
```

## tuple and array assignability

### parenthesized tuples are assignable to dynamic arrays when elements are compatible

> Tuples are assignable to dynamic arrays when element types are compatible.

```ds
const pair: (int32, int32) = (1, 2);
const values: int32[] = pair;
```

### bracket tuples are assignable to dynamic arrays when elements are compatible

> Bracket tuples are assignable to dynamic arrays when element types are compatible.

```ds
const pair: [int32, int32] = [1, 2];
const values: int32[] = pair;
```

### tuples reject assignment to incompatible dynamic arrays

> Tuples reject assignment to dynamic arrays with incompatible element types.

```ds
const pair: (int32, int32) = (1, 2);
const values: string[] = pair;
```

- not assignable

### dynamic arrays are not assignable to fixed tuples

> Dynamic arrays are not assignable to fixed tuples.

```ds
const values: int32[] = [1, 2];
const pair: (int32, int32) = values;
const pair2: [int32, int32] = values;
```

- not assignable