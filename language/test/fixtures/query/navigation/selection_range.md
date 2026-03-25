# Selection Range

## Basic Expressions

### Selection depth in function body

The cursor is inside a nested expression. Expanding selection should move through: identifier -> expression -> statement -> function body -> function -> module.

```ds
function add(a: int32, b: int32): int32 {
    return $0a + b;
}
```

The cursor is on `a` inside the function. The selection hierarchy includes the identifier, binary expression, return statement, function body, and function declaration.

```query selection_range $0
main.ds:2:12-2:13
main.ds:2:12-2:17
main.ds:2:5-2:17
main.ds:2:5-2:18
main.ds:1:41-3:2
main.ds:1:1-3:2
```

### Selection depth in nested arithmetic

The cursor is inside a nested arithmetic expression with parentheses.
Expanding selection should walk from the identifier to the parenthesized expression and the full return statement.

```ds
function compute(value: int32): int32 {
    return ($0value + 1) * 2;
}
```

The snapshot asserts the full selection chain from leaf to root.

```query selection_range $0
main.ds:2:13-2:18
main.ds:2:13-2:22
main.ds:2:12-2:23
main.ds:2:12-2:27
main.ds:2:5-2:27
main.ds:2:5-2:28
main.ds:1:39-3:2
main.ds:1:1-3:2
```

### Selection depth in object literals

The cursor is inside an object literal value.
Expanding selection should move through the literal, property, object literal, and declaration.

```ds
const obj = {
    bar: $02,
};
```

The snapshot captures the full selection chain for the literal.

```query selection_range $0
main.ds:2:10-2:11
main.ds:2:5-2:11
main.ds:1:13-3:2
main.ds:1:7-3:2
main.ds:1:1-3:2
main.ds:1:1-3:3
```

### Selection depth in member access call

The cursor is on a member access inside a call argument.
Expanding selection should walk from the identifier to the call expression and statement.

```ds
struct User {
    name: string
}

function greet(user: User): void {
    print(user.$0name);
}
```

The snapshot captures the selection chain for the member access.

```query selection_range $0
main.ds:6:11-6:20
main.ds:6:5-6:21
main.ds:6:5-6:22
main.ds:5:34-7:2
main.ds:5:1-7:2
```

### Selection depth in type annotations

The cursor is on a type name inside a variable annotation.
Expanding selection should walk from the identifier to the declaration and module.

```ds
struct Point {
    x: int32
    y: int32
}

const p: $0Point = Point { x: 1, y: 2 };
```

```query selection_range $0
main.ds:6:10-6:15
main.ds:6:7-6:38
main.ds:6:1-6:38
main.ds:6:1-6:39
```

## Damaged Syntax

### Keep selection ranges working after malformed function declarations

Selection ranges should still expand normally for later valid expressions in damaged files.

```ds
function broken(

const value = $01;
```

```query selection_range $0
main.ds:3:15-3:16
main.ds:3:7-3:16
main.ds:3:1-3:16
main.ds:3:1-3:17
```

### Keep selection ranges working after bare new recovery statements

Selection ranges should still expand normally for later valid expressions after one bare `new` recovery statement.

```ds
new

const value = $01;
```

```query selection_range $0
main.ds:3:15-3:16
main.ds:3:7-3:16
main.ds:3:1-3:16
main.ds:3:1-3:17
```

### Keep selection ranges working after malformed call statements

Selection ranges should still expand normally for later valid expressions after one malformed call statement.

```ds
broken(,

const value = $01;
```

```query selection_range $0
main.ds:3:15-3:16
main.ds:3:7-3:16
main.ds:3:1-3:16
main.ds:3:1-3:17
```

### Keep selection ranges working after throw recovery statements

Selection ranges should still expand normally for later valid expressions after one recovered `throw` statement.

```ds
throw

const value = $01;
```

```query selection_range $0
main.ds:3:15-3:16
main.ds:3:7-3:16
main.ds:3:1-3:16
main.ds:3:1-3:17
```

### Keep selection ranges working after yield star recovery statements

Selection ranges should still expand normally for later valid expressions after one recovered `yield*` statement.

```ds
function* broken() {
    yield*
    const skipped = 1;
}

const value = $01;
```

```query selection_range $0
main.ds:6:15-6:16
main.ds:6:7-6:16
main.ds:6:1-6:16
main.ds:6:1-6:17
```
