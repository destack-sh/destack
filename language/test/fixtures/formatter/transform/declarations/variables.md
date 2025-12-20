# Variable Declarations

Tests for variable declaration formatting.

## const

### basic const

Extra whitespace around the assignment should be normalized.

```ds
const   x   =   1
```

The formatter produces a single space around `=` and adds a trailing semicolon.

```ds expected
const x = 1;
```

### const with type annotation

Type annotations have no space before the colon and one space after.

```ds
const   x  :  number   =   1
```

```ds expected
const x: number = 1;
```

## let

### basic let

Mutable variable declarations use `let`.

```ds
let   x   =   1
```

```ds expected
let x = 1;
```

## var

### basic var

Legacy `var` declarations are preserved but follow the same spacing rules.

```ds
^mut   x   =   1
```

```ds expected
^mut x = 1;
```
