# Tuple Types

Tests for tuple type checking.

## Basic Tuples

### two element tuple

> Tuples have fixed length and typed positions.

```ds
const x: [number, string] = [1, "hello"]
```

### three element tuple

> Tuples can have any number of elements.

```ds
const x: [number, string, boolean] = [1, "hello", true]
```

## Tuple Inference

### inferred tuple

> Array literals can be inferred as tuples based on context.

```ds
const x = [1, "hello"]
```
