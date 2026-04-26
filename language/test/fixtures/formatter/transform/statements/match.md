# Match Expressions

Tests for match expression formatting.

## Basic Match

### simple match

Condensed match expressions are expanded with each arm on its own line.

```ds
match(x){1=>"one";2=>"two";_=>"other"}
```

Space is added after `match` and around the scrutinee.

```ds expected
match (x) {
    1 => "one"
    2 => "two"
    _ => "other"
}
```

### match with block bodies

Arms with block bodies have their blocks expanded to multiple lines.

```ds
match (x) { 1 => { process(); "one" }; 2 => { transform(); "two" }; _ => "other" }
```

```ds expected
match (x) {
    1 => {
        process();
        "one"
    }
    2 => {
        transform();
        "two"
    }
    _ => "other"
}
```

### match as expression

Match expressions used as values get a trailing semicolon.

```ds
const result = match (state) { Ready => "go"; Loading => "wait"; _ => "unknown" }
```

```ds expected
const result = match (state) {
    Ready => "go"
    Loading => "wait"
    _ => "unknown"
};
```

## Pattern Matching

### literal patterns

Literal values can be used as patterns.

```ds
match (n) { 0 => "zero"; 1 => "one"; 2 => "two"; _ => "many" }
```

```ds expected
match (n) {
    0 => "zero"
    1 => "one"
    2 => "two"
    _ => "many"
}
```

### string patterns

String patterns normalize to the configured formatter quote style.

```ds
match (s) { "a" => 1; "b" => 2; _ => 0 }
```

```ds expected
match (s) {
    "a" => 1
    "b" => 2
    _ => 0
}
```

### wildcard pattern

The underscore `_` matches any value.

```ds
match (x) { _ => "anything" }
```

```ds expected
match (x) {
    _ => "anything"
}
```

### binding pattern

Identifiers in patterns bind the matched value to a variable.

```ds
match (x) { n => n * 2 }
```

```ds expected
match (x) {
    n => n * 2
}
```

## Tuple Patterns

### tuple destructuring

Tuples can be destructured in match patterns.

```ds
match (point) { (0, 0) => "origin"; (x, 0) => `x-axis at ${x}`; (0, y) => `y-axis at ${y}`; (x, y) => `at (${x}, ${y})` }
```

```ds expected
match (point) {
    (0, 0) => "origin"
    (x, 0) => `x-axis at ${x}`
    (0, y) => `y-axis at ${y}`
    (x, y) => `at (${x}, ${y})`
}
```

### nested tuple

Tuple patterns can be nested.

```ds
match (data) { ((a, b), c) => a + b + c }
```

```ds expected
match (data) {
    ((a, b), c) => a + b + c
}
```

## Object Patterns

### object destructuring

Objects can be destructured in match patterns.

```ds
match (user) { { name: "admin" } => "admin user"; { name, age } => `${name} is ${age}`; _ => "unknown" }
```

```ds expected
match (user) {
    { name: "admin" } => "admin user"
    { name, age } => `${name} is ${age}`
    _ => "unknown"
}
```

### struct pattern

Named struct patterns include the struct name.

```ds
match (point) { Point { x: 0, y: 0 } => "origin"; Point { x, y } => `at (${x}, ${y})` }
```

```ds expected
match (point) {
    Point { x: 0, y: 0 } => "origin"
    Point { x, y } => `at (${x}, ${y})`
}
```

## Array Patterns

### array destructuring

Arrays can be destructured in match patterns.

```ds
match (arr) { [] => "empty"; [a] => `one: ${a}`; [a, b] => `two: ${a}, ${b}`; _ => "many" }
```

```ds expected
match (arr) {
    [] => "empty"
    [a] => `one: ${a}`
    [a, b] => `two: ${a}, ${b}`
    _ => "many"
}
```

### array with rest

Rest patterns capture remaining elements.

```ds
match (arr) { [first, ...rest] => first; [] => null }
```

```ds expected
match (arr) {
    [first, ...rest] => first
    [] => null
}
```

### array first and last

The `..` pattern matches elements in the middle. Multi-element tuple results do not need a trailing comma.

```ds
match (arr) { [first, ..., last] => (first, last); _ => null }
```

```ds expected
match (arr) {
    [first, ..., last] => (first, last)
    _ => null
}
```

## Variant Patterns

### enum variant

Enum variants can be matched with destructuring.

```ds
match (result) { Ok(value) => value; Err(e) => throw e }
```

```ds expected
match (result) {
    Ok(value) => value
    Err(e) => throw e
}
```

### qualified variant

Variants can be qualified with their enum name.

```ds
match (result) { Result.Ok(v) => v; Result.Err(e) => handle(e) }
```

```ds expected
match (result) {
    Result.Ok(v) => v
    Result.Err(e) => handle(e)
}
```

### option pattern

Option types use Some and None variants.

```ds
match (opt) { Some(x) => x; None => default }
```

```ds expected
match (opt) {
    Some(x) => x
    None => default
}
```

## Union Patterns

### or pattern

Multiple patterns can be combined with `|`.

```ds
match (x) { 1 | 2 | 3 => "small"; _ => "other" }
```

```ds expected
match (x) {
    1 | 2 | 3 => "small"
    _ => "other"
}
```

### or pattern with strings

Or patterns work with any pattern type.

```ds
match (s) { "a" | "b" | "c" => true; _ => false }
```

```ds expected
match (s) {
    "a" | "b" | "c" => true
    _ => false
}
```

## Guards

### simple guard

Guard conditions keep their required parentheses.

```ds
match (n) { x if (x > 0) => "positive"; x if (x < 0) => "negative"; _ => "zero" }
```

```ds expected
match (n) {
    x if (x > 0) => "positive"
    x if (x < 0) => "negative"
    _ => "zero"
}
```

### complex guard

Guards can use any boolean expression.

```ds
match (user) { User { age } if (age >= 18) => "adult"; User { age } if (age >= 13) => "teen"; _ => "child" }
```

```ds expected
match (user) {
    User { age } if (age >= 18) => "adult"
    User { age } if (age >= 13) => "teen"
    _ => "child"
}
```

### guard with method call

Method calls work in guard conditions.

```ds
match (x) { v if (v.isValid()) => process(v); _ => null }
```

```ds expected
match (x) {
    v if (v.isValid()) => process(v)
    _ => null
}
```

## Union Patterns

### numeric unions

Union patterns can match multiple literal values in one arm.

```ds
match (n) { 1 | 2 | 3 => "small"; 4 | 5 => "medium"; _ => "large" }
```

```ds expected
match (n) {
    1 | 2 | 3 => "small"
    4 | 5 => "medium"
    _ => "large"
}
```

## Decorated Arms

### decorated match arm

Match arms can have decorators for optimization hints.

```ds
match (event) { @likely Click(pos) => handleClick(pos); @cold Error(e) => logError(e) }
```

Body level arm decorators stay on their own line above the arm.

```ds expected
match (event) {
    @likely
    Click(pos) => handleClick(pos)
    @cold
    Error(e) => logError(e)
}
```
