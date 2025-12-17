# Generics

Tests for generic type argument formatting.

## Type Arguments

### generic brackets have no internal spacing

Spaces inside angle brackets should be removed.

```ds
const x: Array< number > = []
```

```ds expected
const x: Array<number> = [];
```

### generic with multiple parameters

Multiple type parameters are separated by comma and space.

```ds
const x: Map< string , number > = new Map()
```

```ds expected
const x: Map<string, number> = new Map();
```
