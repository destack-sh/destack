# Type Alias Declarations

Tests for type alias declaration formatting.

## Basic Type Aliases

### simple type alias

Extra whitespace around the type alias should be normalized.

```ds
type   Foo   =   number
```

Type aliases have single spaces around `=` and no trailing semicolon.

```ds expected
type Foo = number
```

### type alias with union

Union types have spaces around the `|` operator.

```ds
type   Foo   =   string   |   number
```

```ds expected
type Foo = string | number
```

### type alias with intersection

Intersection types have spaces around the `&` operator.

```ds
type   Foo   =   A   &   B
```

```ds expected
type Foo = A & B
```
