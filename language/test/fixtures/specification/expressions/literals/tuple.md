# Tuple Literals

Tuple literal type inference and checking.

## tuples

### tuple of literals

> Tuples infer element types from their values.

```ds
const x = (1, "two", true);
x satisfies (number, string, boolean);
```

### tuple literals widen element types

> Tuple literals widen element literals without const assertions.

```ds
let pair = (1, 2);
pair satisfies (number, number);
```

### tuple literals do not preserve literal elements

> Tuple literals do not retain literal element types without const assertions.

```ds
let pair = (1, 2);
pair satisfies (1, 2);
```

- contains: not assignable

### const tuples still widen without const assertions

> Const tuple bindings still widen element literals without const assertions.

```ds
const pair = (1, 2);
pair satisfies (number, number);
```

### const tuples do not preserve literal elements without const assertions

> Const tuple bindings do not keep literal element types without const assertions.

```ds
const pair = (1, 2);
pair satisfies (1, 2);
```

- contains: not assignable

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

### nested tuple with const assertion

> Const assertions preserve nested tuple literal element types.

```ds
const nested = (1, (2, 3)) as const;
nested satisfies readonly (1, readonly (2, 3));
```

## contextual tuples

### contextual tuple literal

> Tuple literals use contextual types for element inference.

```ds
const pair: (number, string) = (1, "hi");
pair satisfies (number, string);
```

### contextual tuple literal unions

> Contextual tuple unions preserve their union shapes.

```ds
const pair: (1 | 2, "a" | "b") = (1, "a");
pair satisfies (1 | 2, "a" | "b");
```

### contextual tuple literal unions do not narrow to literals

> Contextual tuple unions do not narrow to literal elements.

```ds
const pair: (1 | 2, "a" | "b") = (1, "a");
pair satisfies (1, "a");
```

- contains: not assignable

### contextual tuple literal mismatch

> Tuple literal elements must satisfy contextual element types.

```ds
const pair: (number, string) = (1, 2);
```

- contains: not assignable
