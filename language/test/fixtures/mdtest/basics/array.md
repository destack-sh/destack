# Array Literals

Tests for array literal type inference and checking.

## Basic Arrays

### array of numbers

> Arrays of numbers infer element type as number.

```ds
const x = [1, 2, 3]
```

### array of strings

> Arrays of strings infer element type as string.

```ds
const x = ["a", "b", "c"]
```

### empty array

> Empty arrays have unknown element type.

```ds
const x = []
```

### mixed array

> Arrays with mixed types infer a union element type.

```ds
const x = [1, "two", true]
```
