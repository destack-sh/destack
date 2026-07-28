# Selection Ranges

## Expressions

### Expand a nested arithmetic expression

An identifier expands through its expression, statement, body, and declaration.

```ds main.ds
function compute(value: int32): int32 {
                                      ^ body:start
^ declaration:start
    return (value + 1) * 2;
            ^^^^^ cursor
            ^^^^^^^^^ sum
           ^^^^^^^^^^^ parentheses
           ^^^^^^^^^^^^^^^ product
    ^^^^^^^^^^^^^^^^^^^^^^ return
}
^ body:end
^ declaration:end
```

```query selection_ranges main.ds#cursor
@selection_ranges.range selection=0 depth=0 range=main.ds#cursor
@selection_ranges.range selection=0 depth=1 range=main.ds#sum
@selection_ranges.range selection=0 depth=2 range=main.ds#parentheses
@selection_ranges.range selection=0 depth=3 range=main.ds#product
@selection_ranges.range selection=0 depth=4 range=main.ds#return
@selection_ranges.range selection=0 depth=5 range=main.ds#body
@selection_ranges.range selection=0 depth=6 range=main.ds#declaration
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
           ^^^^^^^ type_annotation
      ^ declarator:start
                                        ^ declarator:end
^ declaration:start
                                        ^ declaration:end
```

```query selection_ranges main.ds#cursor
@selection_ranges.range selection=0 depth=0 range=main.ds#cursor
@selection_ranges.range selection=0 depth=1 range=main.ds#type_annotation
@selection_ranges.range selection=0 depth=2 range=main.ds#declarator
@selection_ranges.range selection=0 depth=3 range=main.ds#declaration
```

## Object Literals

### Expand an object property value

An object property value expands through its property, object, binding, and declaration.

```ds main.ds
const object = {
               ^ object:start
      ^ declarator:start
^ declaration:start
    value: 2,
           ^ value
         ^^^ field_value
    ^^^^^^^^ field
};
^ object:end
^ declarator:end
^ declaration:end
```

```query selection_ranges main.ds#value
@selection_ranges.range selection=0 depth=0 range=main.ds#value
@selection_ranges.range selection=0 depth=1 range=main.ds#field_value
@selection_ranges.range selection=0 depth=2 range=main.ds#field
@selection_ranges.range selection=0 depth=3 range=main.ds#object
@selection_ranges.range selection=0 depth=4 range=main.ds#declarator
@selection_ranges.range selection=0 depth=5 range=main.ds#declaration
```

## Member Access

### Expand a field access

A field name expands through its access expression, statement, body, and declaration.

```ds main.ds
struct User {
    name: string;
}

function read(user: User): string {
                                  ^ body:start
^ declaration:start
    return user.name;
                ^^^^ name
           ^^^^^^^^^ member
    ^^^^^^^^^^^^^^^^ return
}
^ body:end
^ declaration:end
```

```query selection_ranges main.ds#name
@selection_ranges.range selection=0 depth=0 range=main.ds#name
@selection_ranges.range selection=0 depth=1 range=main.ds#member
@selection_ranges.range selection=0 depth=2 range=main.ds#return
@selection_ranges.range selection=0 depth=3 range=main.ds#body
@selection_ranges.range selection=0 depth=4 range=main.ds#declaration
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
                 ^ arguments:start
                               ^ arguments:end
              ^ call:start
                               ^ call:end
      ^ declarator:start
                               ^ declarator:end
^ declaration:start
                               ^ declaration:end
```

```query selection_ranges main.ds#argument
@selection_ranges.range selection=0 depth=0 range=main.ds#argument
@selection_ranges.range selection=0 depth=1 range=main.ds#arguments
@selection_ranges.range selection=0 depth=2 range=main.ds#call
@selection_ranges.range selection=0 depth=3 range=main.ds#declarator
@selection_ranges.range selection=0 depth=4 range=main.ds#declaration
```

### Expand a constructor argument

A constructor argument expands through the argument list, construction, declarator, and declaration.

```ds main.ds
class Box {
    constructor(value: int32) {}
}

const boxed = new Box(1);
                      ^ argument
                     ^^^ arguments
              ^^^^^^^^^^ construction
      ^^^^^^^^^^^^^^^^^^ declarator
^^^^^^^^^^^^^^^^^^^^^^^^ declaration
```

```query selection_ranges main.ds#argument
@selection_ranges.range selection=0 depth=0 range=main.ds#argument
@selection_ranges.range selection=0 depth=1 range=main.ds#arguments
@selection_ranges.range selection=0 depth=2 range=main.ds#construction
@selection_ranges.range selection=0 depth=3 range=main.ds#declarator
@selection_ranges.range selection=0 depth=4 range=main.ds#declaration
```

## Patterns

### Expand a match binding

A binding expands through its pattern, match arm, match expression, declarator, and declaration.

```ds main.ds
declare const pair: (int32, int32);

const total = match (pair) {
              ^ match:start
      ^ declarator:start
^ declaration:start
    (left, right) => left + right
     ^^^^ binding
    ^^^^^^^^^^^^^ pattern
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ arm
};
^ match:end
^ declarator:end
^ declaration:end
```

```query selection_ranges main.ds#binding
@selection_ranges.range selection=0 depth=0 range=main.ds#binding
@selection_ranges.range selection=0 depth=1 range=main.ds#pattern
@selection_ranges.range selection=0 depth=2 range=main.ds#arm
@selection_ranges.range selection=0 depth=3 range=main.ds#match
@selection_ranges.range selection=0 depth=4 range=main.ds#declarator
@selection_ranges.range selection=0 depth=5 range=main.ds#declaration
```

## Lexical Tokens

### Select a complete string literal

A position inside a string selects the complete literal before its declaration.

```ds main.ds
const greeting = "hello";
                  ^^^^^ string_content
                 ^^^^^^^ string
      ^ declarator:start
                       ^ declarator:end
^ declaration:start
                       ^ declaration:end
```

```query selection_ranges main.ds#string_content
@selection_ranges.range selection=0 depth=0 range=main.ds#string
@selection_ranges.range selection=0 depth=1 range=main.ds#declarator
@selection_ranges.range selection=0 depth=2 range=main.ds#declaration
```

### Select a complete comment

A position inside a comment selects the complete comment.

```ds main.ds
// explain the value
^^^^^^^^^^^^^^^^^^^^ comment
               ^^^^^ comment_word
const value = 1;
```

```query selection_ranges main.ds#comment_word
@selection_ranges.range selection=0 depth=0 range=main.ds#comment
```

## Whitespace

### Select the enclosing declaration from whitespace

Whitespace begins at its enclosing source range.

```ds main.ds
const value = 1;
^^^^^^^^^^^^^^^ declaration
     ^ whitespace
```

```query selection_ranges main.ds#whitespace
@selection_ranges.range selection=0 depth=0 range=main.ds#declaration
```

## Multiple Positions

### Expand each requested position

Each requested position returns its own ordered selection chain.

```ds main.ds
const first = 1;
              ^ first
      ^^^^^^^^^ first_declarator
^^^^^^^^^^^^^^^ first_declaration
const second = 2;
               ^ second
      ^^^^^^^^^^ second_declarator
^^^^^^^^^^^^^^^^ second_declaration
```

```query selection_ranges main.ds#first main.ds#second
@selection_ranges.range selection=0 depth=0 range=main.ds#first
@selection_ranges.range selection=0 depth=1 range=main.ds#first_declarator
@selection_ranges.range selection=0 depth=2 range=main.ds#first_declaration
@selection_ranges.range selection=1 depth=0 range=main.ds#second
@selection_ranges.range selection=1 depth=1 range=main.ds#second_declarator
@selection_ranges.range selection=1 depth=2 range=main.ds#second_declaration
```
