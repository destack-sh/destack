# Tuple Literals

> NOTE #Incomplete: implement/mdtest tuple literals

Explicit tuple syntax with parentheses.

## Coverage

- **Tuple literals**: `(a, b, c)`
- **Tuple types**: `(int32, string, boolean)`
- **Named tuples**: `(x: int32, y: int32)`
- **Destructuring**: `const (x, y) = point`
- **Single-element**: `(x,)` to distinguish from grouping

## Example

```ds
const point: (int32, int32) = (1, 2);
const (x, _) = getPoint();

type Coordinate = (x: float32, y: float32);
```

In `.ds` files, `(a, b, c)` is a tuple, not the comma operator.
