# Type Sets

## Union Types

### union inside intersection uses parentheses

Unions inside intersections are parenthesized.

```ds
type Combined = A & (B | C)
```

```ds expected
type Combined = A & (B | C);
```

## Intersection Types

### intersection inside union uses parentheses

Intersections inside unions are parenthesized.

```ds
type Combined = A | (B & C)
```

```ds expected
type Combined = A | (B & C);
```

