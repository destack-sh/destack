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

### intersection inside union omits redundant parentheses

Intersections inside unions can omit redundant parentheses.

```ds
type Combined = A | (B & C)
```

```ds expected
type Combined = A | B & C;
```
