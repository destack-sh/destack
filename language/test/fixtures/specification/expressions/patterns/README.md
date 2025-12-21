# Patterns

> NOTE #Incomplete: implement/mdtest pattern syntax

Pattern syntax for destructuring and matching.

## Coverage

- **Wildcard**: `_` ignores a value
- **Binding**: `x` or `var x` captures a value
- **Literal**: `42`, `"hello"`, `true` match exact values
- **Tuple**: `(a, b)` destructures tuples
- **Object**: `{ x, y }` or `Point { x, y }` destructures objects
- **Array**: `[a, b, ...rest]` destructures arrays
- **Union**: `1 | 2 | 3` matches any of several patterns
- **Range**: `1..10` matches values in range
- **Guard**: `x if x > 0` adds conditions

## Example

```ds
match (value) {
    0 => "zero"
    1 | 2 | 3 => "small"
    n if n < 0 => "negative"
    _ => "other"
}

const { x, y: vertical } = point;
const [first, ...rest] = array;
const (a, _, c) = tuple;
```
