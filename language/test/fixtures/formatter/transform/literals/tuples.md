# Tuple Literals

Tests for Destack tuple literal formatting.

## Basic Tuples

### simple tuple with trailing comma

Tuple literals get a trailing comma to distinguish from parenthesized expressions.

```ds
( 1 , 2 , 3 )
```

```ds expected
(1, 2, 3,);
```

### tuple destructuring

Tuple patterns in destructuring follow the same spacing rules.

```ds
const ( a , b ) = getTuple()
```

```ds expected
const (a, b) = getTuple();
```
