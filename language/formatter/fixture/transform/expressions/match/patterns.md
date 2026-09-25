# Match Patterns

## Match Arms

### compact match

Condensed match expressions are expanded with each arm on its own line.

```tspp
match(x){1=>"one";2=>"two";_=>"other"}
```

Space is added after `match` and around the scrutinee.

```tspp expected
match (x) {
    1 => "one"
    2 => "two"
    _ => "other"
}
```

### match with block bodies

Arms with block bodies have their blocks expanded to multiple lines.

```tspp
match (x) { 1 => { process(); "one" }; 2 => { transform(); "two" }; _ => "other" }
```

```tspp expected
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

```tspp
const result = match (state) { Ready => "go"; Loading => "wait"; _ => "unknown" }
```

```tspp expected
const result = match (state) {
    Ready => "go"
    Loading => "wait"
    _ => "unknown"
};
```

## Pattern Matching

### literal patterns

Literal values can be used as patterns.

```tspp
match (n) { 0 => "zero"; 1 => "one"; 2 => "two"; _ => "many" }
```

```tspp expected
match (n) {
    0 => "zero"
    1 => "one"
    2 => "two"
    _ => "many"
}
```

### string patterns

String patterns normalize to the configured formatter quote style.

```tspp
match (s) { "a" => 1; "b" => 2; _ => 0 }
```

```tspp expected
match (s) {
    "a" => 1
    "b" => 2
    _ => 0
}
```

### wildcard pattern

The underscore `_` matches any value.

```tspp
match (x) { _ => "anything" }
```

```tspp expected
match (x) {
    _ => "anything"
}
```

### binding pattern

Identifiers in patterns bind the matched value to a variable.

```tspp
match (x) { n => n * 2 }
```

```tspp expected
match (x) {
    n => n * 2
}
```

## Decorated Arms

### decorated match arm

Match arms can have decorators for optimization hints.

```tspp
match (event) { @likely Click(pos) => handleClick(pos); @cold Error(e) => logError(e) }
```

Body level arm decorators stay on their own line above the arm.

```tspp expected
match (event) {
    @likely
    Click(pos) => handleClick(pos)
    @cold
    Error(e) => logError(e)
}
```

## Nested Patterns

### nested destructuring guard with block tail

Nested tagged, array, and object patterns compose with guards and block tail expressions.

```tspp line-width=120
const score = match (packet) { Event.Batch([first, ..., last], { meta: { id = fallbackId }, ...rest }) if (rest.valid) => { log(id); first + last + id }; Event.Single(Point { x, y }) => x + y; _ => 0 }
```

```tspp expected
const score = match (packet) {
    Event.Batch(
        [first, ..., last],
        {
            meta: { id = fallbackId },
            ...rest
        },
    ) if (rest.valid) => {
        log(id);
        first + last + id
    }
    Event.Single(Point { x, y }) => x + y
    _ => 0
};
```
