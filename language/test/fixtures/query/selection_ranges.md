# Selection Ranges

## Expressions

### Expand a nested arithmetic expression

The selected identifier expands through its expression, statement, body, and declaration.

```ds main.ds
function compute(value: int32): int32 {
    return (value + 1) * 2;
            ^^^^^ cursor
}
```

```query selection_ranges main.ds#cursor
@selection_ranges.range selection=0 depth=0 range=main.ds#cursor
@selection_ranges.range selection=0 depth=1 range=main.ds:2:13-2:22
@selection_ranges.range selection=0 depth=2 range=main.ds:2:12-2:23
@selection_ranges.range selection=0 depth=3 range=main.ds:2:12-2:27
@selection_ranges.range selection=0 depth=4 range=main.ds:2:5-2:27
@selection_ranges.range selection=0 depth=5 range=main.ds:1:39-3:2
@selection_ranges.range selection=0 depth=6 range=main.ds:1:1-3:2
```

## Type Annotations

### Expand a type annotation

A type name expands through its declarator and declaration.

```ds main.ds
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
             ^^^^^ cursor
```

```query selection_ranges main.ds#cursor
@selection_ranges.range selection=0 depth=0 range=main.ds#cursor
@selection_ranges.range selection=0 depth=1 range=main.ds:6:7-6:42
@selection_ranges.range selection=0 depth=2 range=main.ds:6:1-6:42
```

## Object Literals

### Expand an object property value

An object property value expands through its property, object, binding, and declaration.

```ds main.ds
const object = {
    value: 2,
           ^ value
};
```

```query selection_ranges main.ds#value
@selection_ranges.range selection=0 depth=0 range=main.ds#value
@selection_ranges.range selection=0 depth=1 range=main.ds:2:5-2:13
@selection_ranges.range selection=0 depth=2 range=main.ds:1:16-3:2
@selection_ranges.range selection=0 depth=3 range=main.ds:1:7-3:2
@selection_ranges.range selection=0 depth=4 range=main.ds:1:1-3:2
```

## Member Access

### Expand a field access

A field name expands through its access expression, statement, body, and declaration.

```ds main.ds
struct User {
    name: string;
}

function read(user: User): string {
    return user.name;
                ^^^^ name
}
```

```query selection_ranges main.ds#name
@selection_ranges.range selection=0 depth=0 range=main.ds#name
@selection_ranges.range selection=0 depth=1 range=main.ds:6:12-6:21
@selection_ranges.range selection=0 depth=2 range=main.ds:6:5-6:21
@selection_ranges.range selection=0 depth=3 range=main.ds:5:35-7:2
@selection_ranges.range selection=0 depth=4 range=main.ds:5:1-7:2
```

## Calls

### Expand a call argument

An argument expands through its argument list, call, declarator, and declaration.

```ds main.ds
declare function add(left: int32, right: int32): int32;
const first = 1;
const second = 2;
const total = add(first, second);
                  ^^^^^ argument
```

```query selection_ranges main.ds#argument
@selection_ranges.range selection=0 depth=0 range=main.ds#argument
@selection_ranges.range selection=0 depth=1 range=main.ds:4:18-4:33
@selection_ranges.range selection=0 depth=2 range=main.ds:4:15-4:33
@selection_ranges.range selection=0 depth=3 range=main.ds:4:7-4:33
@selection_ranges.range selection=0 depth=4 range=main.ds:4:1-4:33
```

## Patterns

### Expand a match binding

A binding expands through its pattern, match arm, match expression, declarator, and declaration.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
    (left, right) => left + right
     ^^^^ binding
};
```

```query selection_ranges main.ds#binding
@selection_ranges.range selection=0 depth=0 range=main.ds#binding
@selection_ranges.range selection=0 depth=1 range=main.ds:4:5-4:18
@selection_ranges.range selection=0 depth=2 range=main.ds:4:5-4:34
@selection_ranges.range selection=0 depth=3 range=main.ds:3:15-5:2
@selection_ranges.range selection=0 depth=4 range=main.ds:3:7-5:2
@selection_ranges.range selection=0 depth=5 range=main.ds:3:1-5:2
```

## Lexical Tokens

### Select a complete string literal

A position inside a string selects the complete literal before its declaration.

```ds main.ds
const greeting = "hello";
                  ^^^^^ string_content
```

```query selection_ranges main.ds#string_content
@selection_ranges.range selection=0 depth=0 range=main.ds:1:18-1:25
@selection_ranges.range selection=0 depth=1 range=main.ds:1:7-1:25
@selection_ranges.range selection=0 depth=2 range=main.ds:1:1-1:25
```

### Select a complete comment

A position inside a comment selects the complete comment.

```ds main.ds
// explain the value
               ^^^^^ comment_word
const value = 1;
```

```query selection_ranges main.ds#comment_word
@selection_ranges.range selection=0 depth=0 range=main.ds:1:1-1:21
```

## Whitespace

### Select the enclosing declaration from whitespace

Whitespace does not create a synthetic leaf range.

```ds main.ds
const value = 1;
     ^ whitespace
```

```query selection_ranges main.ds#whitespace
@selection_ranges.range selection=0 depth=0 range=main.ds:1:1-1:16
```

## Multiple Positions

### Expand each requested position

Each requested position retains its own ordered selection chain.

```ds main.ds
const first = 1;
              ^ first
const second = 2;
               ^ second
```

```query selection_ranges main.ds#first main.ds#second
@selection_ranges.range selection=0 depth=0 range=main.ds#first
@selection_ranges.range selection=0 depth=1 range=main.ds:1:7-1:16
@selection_ranges.range selection=0 depth=2 range=main.ds:1:1-1:16
@selection_ranges.range selection=1 depth=0 range=main.ds#second
@selection_ranges.range selection=1 depth=1 range=main.ds:2:7-2:17
@selection_ranges.range selection=1 depth=2 range=main.ds:2:1-2:17
```
