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

### const with object destructuring

Object patterns keep brace spacing and commas.

```ds
const {a,b} = value
```

```ds expected
const { a, b } = value;
```

### const with array destructuring

Array patterns keep tight brackets.

```ds
const [a, b] = tuple
```

```ds expected
const [a, b] = tuple;
```

### const with rest destructuring

Rest patterns keep tight spacing.

```ds
const { a, ...rest } = value
```

```ds expected
const { a, ...rest } = value;
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

### let with multiple declarators

Multiple declarators use commas and spacing.

```ds
let a=1, b=2, c=3
```

```ds expected
let a = 1, b = 2, c = 3;
```

## var

### basic var

Legacy `var` declarations are preserved but follow the same spacing rules.

```ds
var   x   =   1
```

```ds expected
var x = 1;
```
