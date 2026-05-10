# Tuple Literals

Tuple fixtures cover tuple literals, tuple patterns, and comments.

## Tuple Forms

### tuple literal

Multi-element tuple literals do not need an extra trailing comma.

```ds
( 1 , 2 , 3 )
```

```ds expected
(1, 2, 3);
```

### tuple destructuring

Tuple patterns in destructuring follow the same spacing rules.

```ds
const ( a , b ) = getTuple()
```

```ds expected
const (a, b) = getTuple();
```

## Comments

### tuple element comments

Tuple element comments keep trailing and leading ownership separate.

```ds
const value = (first, // first
// second
second)
```

```ds expected
const value = (
    first, // first
    // second
    second,
);
```

### tuple pattern comments

Tuple pattern comments stay with the corresponding fields.

```ds
const (first, /* middle */ second, ...rest) = getTuple()
```

```ds expected
const (first, /* middle */ second, ...rest) = getTuple();
```
