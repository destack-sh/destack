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

Type annotations should have no space before the colon and one space after.

```ds
const   x  :  number   =   1
```

```ds expected
const x: number = 1;
```

## let

### basic let

```ds
let   x   =   1
```

```ds expected
let x = 1;
```

## var

### basic var

```ds
var   x   =   1
```

```ds expected
var x = 1;
```
