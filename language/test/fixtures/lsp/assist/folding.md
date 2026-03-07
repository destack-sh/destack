# Folding Range

## Basic Declarations

### Fold multi-line declarations

Folding ranges should cover multi-line declarations and block comments.

```ds:main.ds
function add(a: int, b: int): int {
    return a + b;
}

class Point {
    x: int;
    y: int;
}

/*
block
comment
*/
```

```lsp folding_range
range=0:0-2:0

range=4:0-7:0
```

## Folding churn

### Add a second multiline function fold
Folding ranges should grow when a second multiline declaration is inserted.

```ds:main.ds
function first(): int32 {
   return 1;
}
```

```ds:main.ds[1]
function first(): int32 {
   return 1;
}
function second(): int32 {
   return 2;
}
```

```lsp folding_range main.ds [0]
range=0:0-2:0
```

```lsp folding_range main.ds [1]
range=0:0-2:0

range=3:0-5:0
```

### Add a class fold beside an existing function fold
Folding ranges should include the new class block after the overlay grows the file.

```ds:main.ds
function first(): int32 {
   return 1;
}
```

```ds:main.ds[1]
function first(): int32 {
   return 1;
}
class Point {
   x: int32;
   y: int32;
}
```

```lsp folding_range main.ds [0]
range=0:0-2:0
```

```lsp folding_range main.ds [1]
range=0:0-2:0

range=3:0-6:0
```
