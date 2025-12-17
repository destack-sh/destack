# Type Alias Declarations

Tests for type alias declaration formatting.

## Basic Type Aliases

### simple type alias

Extra whitespace around the type alias should be normalized.

```ds
type   Foo   =   number
```

Type aliases have single spaces around `=` and a trailing semicolon.

```ds expected
type Foo = number;
```

### type alias with union

Union types have spaces around the `|` operator.

```ds
type   Foo   =   string   |   number
```

```ds expected
type Foo = string | number;
```

### type alias with intersection

Intersection types have spaces around the `&` operator.

```ds
type   Foo   =   A   &   B
```

```ds expected
type Foo = A & B;
```

## Multi-line Type Unions

### long union breaks at operators

When a union type exceeds line width, it breaks with operators at the start of lines.

```ds line-width=30
type Result = Success | Failure | Pending | Unknown
```

```ds expected
type Result = Success
    | Failure
    | Pending
    | Unknown;
```

### long intersection breaks at operators

Intersection types also break with operators at the start of lines.
(Unlike in TypeScript, we can't lead with `&` because it's a valid unary operator i.e. references.)

```ds line-width=30
type Combined = HasName & HasAge & HasEmail
```

```ds expected
type Combined = HasName
    & HasAge
    & HasEmail;
```
