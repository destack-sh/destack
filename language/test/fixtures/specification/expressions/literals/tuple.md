# Tuple Literals

Tests for tuple literal type inference and checking.

## Basic Tuples

### tuple of literals

> Tuples infer element types from their values.

```ds
const x = (1, "two", true);
x satisfies (number, string, boolean);
```

### nested tuple

> Tuples can contain nested tuples.

```ds
const x = (1, (2, 3));
x satisfies (number, (number, number));
```

### tuple with const assertion

> Const assertions preserve tuple literal element types.

```ds
const x = (1, "two", true) as const;
x satisfies readonly (1, "two", true);
```

## Contextual Tuples

### contextual tuple literal

> Tuple literals use contextual types for element inference.

```ds
const pair: (number, string) = (1, "hi");
pair satisfies (number, string);
```

### contextual tuple literal mismatch

> Tuple literal elements must satisfy contextual element types.

```ds
const pair: (number, string) = (1, 2);
```

- contains: type (number, int32) is not assignable to type (number, string)
