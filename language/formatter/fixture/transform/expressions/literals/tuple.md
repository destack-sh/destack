# Tuple Literals

Tuple fixtures cover tuple literals, tuple patterns, and comments.

## Tuple Forms

### tuple literal

Multi-element tuple literals do not need an extra trailing comma.

```tspp
( 1 , 2 , 3 )
```

```tspp expected
(1, 2, 3);
```

### tuple destructuring

Tuple patterns in destructuring follow the same spacing rules.

```tspp
const ( a , b ) = getTuple()
```

```tspp expected
const (a, b) = getTuple();
```

## Comments

### tuple element comments

Tuple element comments keep trailing and leading ownership separate.

```tspp
const value = (first, // first
// second
second)
```

```tspp expected
const value = (
    first, // first
    // second
    second,
);
```

### tuple pattern comments

Tuple pattern comments stay with the corresponding fields.

```tspp
const (first, /* middle */ second, ...rest) = getTuple()
```

```tspp expected
const (first, /* middle */ second, ...rest) = getTuple();
```
