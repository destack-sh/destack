
## Expressions

### Expand a nested arithmetic expression

An identifier expands through its expression, statement, body, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#cursor
@selection_ranges.range selection=0 depth=0 range=main.tspp#cursor
@selection_ranges.range selection=0 depth=1 range=main.tspp#sum
@selection_ranges.range selection=0 depth=2 range=main.tspp#parentheses
@selection_ranges.range selection=0 depth=3 range=main.tspp#product
@selection_ranges.range selection=0 depth=4 range=main.tspp#return
@selection_ranges.range selection=0 depth=5 range=main.tspp#body
@selection_ranges.range selection=0 depth=6 range=main.tspp#declaration
```

### Preserve expression ancestry after preceding text changes

Selection ranges follow the selected expression after its offsets move.

```tspp main.tspp
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

```query selection_ranges main.tspp#cursor
@selection_ranges.range selection=0 depth=0 range=main.tspp#cursor
@selection_ranges.range selection=0 depth=1 range=main.tspp#sum
@selection_ranges.range selection=0 depth=2 range=main.tspp#parentheses
@selection_ranges.range selection=0 depth=3 range=main.tspp#product
@selection_ranges.range selection=0 depth=4 range=main.tspp#return
@selection_ranges.range selection=0 depth=5 range=main.tspp#body
@selection_ranges.range selection=0 depth=6 range=main.tspp#declaration
```

```tspp main.tspp change
// keep this calculation explicit
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

```query selection_ranges main.tspp#cursor
@selection_ranges.range selection=0 depth=0 range=main.tspp#cursor
@selection_ranges.range selection=0 depth=1 range=main.tspp#sum
@selection_ranges.range selection=0 depth=2 range=main.tspp#parentheses
@selection_ranges.range selection=0 depth=3 range=main.tspp#product
@selection_ranges.range selection=0 depth=4 range=main.tspp#return
@selection_ranges.range selection=0 depth=5 range=main.tspp#body
@selection_ranges.range selection=0 depth=6 range=main.tspp#declaration
```

## Type Annotations

### Expand a type annotation

A type name expands through its declarator and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#cursor
@selection_ranges.range selection=0 depth=0 range=main.tspp#cursor
@selection_ranges.range selection=0 depth=1 range=main.tspp#type_annotation
@selection_ranges.range selection=0 depth=2 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=3 range=main.tspp#declaration
```

## Object Literals

### Expand an object property value

An object property value expands through its property, object, binding, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#value
@selection_ranges.range selection=0 depth=0 range=main.tspp#value
@selection_ranges.range selection=0 depth=1 range=main.tspp#field_value
@selection_ranges.range selection=0 depth=2 range=main.tspp#field
@selection_ranges.range selection=0 depth=3 range=main.tspp#object
@selection_ranges.range selection=0 depth=4 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=5 range=main.tspp#declaration
```

## Member Access

### Expand a field access

A field name expands through its access expression, statement, body, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#name
@selection_ranges.range selection=0 depth=0 range=main.tspp#name
@selection_ranges.range selection=0 depth=1 range=main.tspp#member
@selection_ranges.range selection=0 depth=2 range=main.tspp#return
@selection_ranges.range selection=0 depth=3 range=main.tspp#body
@selection_ranges.range selection=0 depth=4 range=main.tspp#declaration
```

## Calls

### Expand a call argument

An argument expands through its argument list, call, declarator, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#argument
@selection_ranges.range selection=0 depth=0 range=main.tspp#argument
@selection_ranges.range selection=0 depth=1 range=main.tspp#arguments
@selection_ranges.range selection=0 depth=2 range=main.tspp#call
@selection_ranges.range selection=0 depth=3 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=4 range=main.tspp#declaration
```

### Expand a constructor argument

A constructor argument expands through the argument list, construction, declarator, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#argument
@selection_ranges.range selection=0 depth=0 range=main.tspp#argument
@selection_ranges.range selection=0 depth=1 range=main.tspp#arguments
@selection_ranges.range selection=0 depth=2 range=main.tspp#construction
@selection_ranges.range selection=0 depth=3 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=4 range=main.tspp#declaration
```

## Patterns

### Expand a match binding

A binding expands through its pattern, match arm, match expression, declarator, and declaration.

```tspp main.tspp
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

```query selection_ranges main.tspp#binding
@selection_ranges.range selection=0 depth=0 range=main.tspp#binding
@selection_ranges.range selection=0 depth=1 range=main.tspp#pattern
@selection_ranges.range selection=0 depth=2 range=main.tspp#arm
@selection_ranges.range selection=0 depth=3 range=main.tspp#match
@selection_ranges.range selection=0 depth=4 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=5 range=main.tspp#declaration
```

## Strings and Comments

### Select a complete string literal

A position inside a string selects the complete literal before its declaration.

```tspp main.tspp
const greeting = "hello";
                  ^^^^^ string_content
                 ^^^^^^^ string
      ^ declarator:start
                       ^ declarator:end
^ declaration:start
                       ^ declaration:end
```

```query selection_ranges main.tspp#string_content
@selection_ranges.range selection=0 depth=0 range=main.tspp#string
@selection_ranges.range selection=0 depth=1 range=main.tspp#declarator
@selection_ranges.range selection=0 depth=2 range=main.tspp#declaration
```

### Select a complete comment

A position inside a comment selects the complete comment.

```tspp main.tspp
// explain the value
^^^^^^^^^^^^^^^^^^^^ comment
               ^^^^^ comment_word
const value = 1;
```

```query selection_ranges main.tspp#comment_word
@selection_ranges.range selection=0 depth=0 range=main.tspp#comment
```

## Whitespace

### Select the enclosing declaration from whitespace

Whitespace begins at its enclosing source range.

```tspp main.tspp
const value = 1;
^^^^^^^^^^^^^^^ declaration
     ^ whitespace
```

```query selection_ranges main.tspp#whitespace
@selection_ranges.range selection=0 depth=0 range=main.tspp#declaration
```

## Multiple Positions

### Expand each requested position

Each requested position returns its own ordered selection chain.

```tspp main.tspp
const first = 1;
              ^ first
      ^^^^^^^^^ first_declarator
^^^^^^^^^^^^^^^ first_declaration
const second = 2;
               ^ second
      ^^^^^^^^^^ second_declarator
^^^^^^^^^^^^^^^^ second_declaration
```

```query selection_ranges main.tspp#first main.tspp#second
@selection_ranges.range selection=0 depth=0 range=main.tspp#first
@selection_ranges.range selection=0 depth=1 range=main.tspp#first_declarator
@selection_ranges.range selection=0 depth=2 range=main.tspp#first_declaration
@selection_ranges.range selection=1 depth=0 range=main.tspp#second
@selection_ranges.range selection=1 depth=1 range=main.tspp#second_declarator
@selection_ranges.range selection=1 depth=2 range=main.tspp#second_declaration
```
