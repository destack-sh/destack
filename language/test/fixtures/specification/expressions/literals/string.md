# String Literals

String literal type inference and checking.

## strings

### string literal

> String literals can be assigned to string type.

```ds
const x: string = "hello";
```

### empty string

> Empty strings are valid string literals.

```ds
const x: string = "";
```

### string with spaces

> Strings can contain spaces.

```ds
const x: string = "hello world";
```

## Inference

### inferred string type

> String literals without annotation infer to string.

```ds
const x = "hello";
x satisfies string;
```

### inferred empty string

> Empty string literals infer to string.

```ds
const x = "";
x satisfies string;
```

### string does not satisfy number

> String literal cannot satisfy number type.

```ds
const x = "hello";
x satisfies number;
```

- contains: expected number, found "hello"

## String Members
