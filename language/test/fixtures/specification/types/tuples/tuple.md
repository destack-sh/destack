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

## tuple optional ordering

### optional elements must be last

> Optional tuple elements must be last in the tuple.

```ts
type Bad = [string?, number];
type BadLabeled = [x?: number, y: string];
```

- contains: optional tuple elements must be last
