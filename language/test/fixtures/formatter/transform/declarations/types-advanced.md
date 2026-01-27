# Advanced Type Formatting

Tests for complex type expressions and type operators.

## Function Types

### function type alias

Function types keep parameter and return type spacing.

```ds
type Predicate = (value: string) => boolean
```

```ds expected
type Predicate = (value: string) => boolean;
```

## Indexed Access and Queries

### indexed access type

Indexed access types keep brackets tight.

```ds
type Name = User["name"]
```

```ds expected
type Name = User["name"];
```

### type query

Type queries keep a space after `typeof`.

```ds
type Result = typeof someValue
```

```ds expected
type Result = typeof someValue;
```

## Conditional Types

### conditional with union branches

Conditional types format with spaces around `?` and `:`.

```ds
type Maybe<T> = T extends string ? T | null : T
```

```ds expected
type Maybe<T> = T extends string ? T | null : T;
```

### nested conditional type

Nested conditional types preserve parentheses.

```ds
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
```

```ds expected
type Nested<T> = T extends string ? (T extends 'a' ? 1 : 2) : 3;
```

## Intersections and Unions

### union inside intersection uses parentheses

Unions inside intersections are parenthesized.

```ds
type Combined = A & (B | C)
```

```ds expected
type Combined = A & (B | C);
```

### intersection inside union uses parentheses

Intersections inside unions are parenthesized.

```ds
type Combined = A | (B & C)
```

```ds expected
type Combined = A | (B & C);
```
