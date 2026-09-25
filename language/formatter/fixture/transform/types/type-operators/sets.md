# Type Sets

## Union Types

### union inside intersection uses parentheses

Unions inside intersections are parenthesized.

```tspp
type Combined = A & (B | C)
```

```tspp expected
type Combined = A & (B | C);
```

## Intersection Types

### intersection inside union omits redundant parentheses

Intersections inside unions can omit redundant parentheses.

```tspp
type Combined = A | (B & C)
```

```tspp expected
type Combined = A | B & C;
```
