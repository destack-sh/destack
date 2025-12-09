# String Literals

Tests for string literal type inference and checking.

## Basic Strings

### string literal

> String literals can be assigned to string type.

```ds
const x: string = "hello"
```

### empty string

> Empty strings are valid string literals.

```ds
const x: string = ""
```

### string with spaces

> Strings can contain spaces.

```ds
const x: string = "hello world"
```

## Inference

### inferred string type

> String literals without annotation infer to string.

```ds
const x = "hello"
```

### inferred empty string

> Empty string literals infer to string.

```ds
const x = ""
```
