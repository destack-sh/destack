# Tuple Literals

Tests for Destack tuple literal formatting.

## Basic Tuples

### simple tuple

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
