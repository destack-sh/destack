# Match and Switch Expressions

Tests for match and switch expression formatting, ensuring the formatter preserves the keyword used.

## Match Expressions

### basic match expression

Match expressions use arrow syntax for cases.

```ds
match (x) { 1 => "one"; 2 => "two" }
```

```ds expected
match (x) {
    1 => "one"
    2 => "two"
}
```

### match with wildcard pattern

Wildcard patterns use underscore in match expressions.

```ds
match (value) { Some(x) => x; _ => defaultValue }
```

```ds expected
match (value) {
    Some(x) => x
    _ => defaultValue
}
```

### match with guard

Match cases can have guards.

```ds
match (x) { n if (n > 0) => "positive"; _ => "non-positive" }
```

```ds expected
match (x) {
    n if (n > 0) => "positive"
    _ => "non-positive"
}
```

### match with annotated arm

Annotations can appear on match arms and stay on their own line above the arm.

```ds
match (result) { @cold Err(e) => handle(e); Ok(v) => v }
```

```ds expected
match (result) {
    @cold
    Err(e) => handle(e)
    Ok(v) => v
}
```

### match with block body

Match cases can have block bodies.

```ds
match (result) { Ok(value) => { process(value); value }; Err(e) => { log(e); null } }
```

```ds expected
match (result) {
    Ok(value) => {
        process(value);
        value
    }
    Err(e) => {
        log(e);
        null
    }
}
```

### match with object expression body

Object expression bodies keep parentheses after the case arrow.

```ds
match (result) { Ok { value } => ({ kind: "ok", value }); Err { error } => ({ kind: "err", error }) }
```

```ds expected
match (result) {
    Ok { value } => ({ kind: "ok", value })
    Err { error } => ({ kind: "err", error })
}
```

### match as expression value

Match expression used as a value gets trailing semicolon.

```ds
const x = match (status) { Success => 1; Failure => 0 }
```

```ds expected
const x = match (status) {
    Success => 1
    Failure => 0
};
```

## Switch Expressions

### basic switch expression

Switch expressions use case/colon syntax.

```ds
switch (x) {
    case 1: "one"
    case 2: "two"
}
```

```ds expected
switch (x) {
    case 1:
        "one";
    case 2:
        "two";
}
```

### switch with default case

Default cases use the default keyword.

```ds
switch (value) {
    case 1: "one"
    default: "other"
}
```

```ds expected
switch (value) {
    case 1:
        "one";
    default:
        "other";
}
```

### switch with string patterns

Switch works with string literal patterns.

```ds
switch (type) {
    case "static": handleStatic()
    case "dynamic": handleDynamic()
    default: handleDefault()
}
```

```ds expected
switch (type) {
    case "static":
        handleStatic();
    case "dynamic":
        handleDynamic();
    default:
        handleDefault();
}
```

## Syntax Preservation

### match preserves match keyword

The formatter should not convert match to switch.

```ds
match (status) { Success => "ok"; Failure => "error" }
```

```ds expected
match (status) {
    Success => "ok"
    Failure => "error"
}
```

### switch preserves switch keyword at statement level

The formatter should not convert switch to match.

```ds
switch (status) {
    case "success": handleSuccess()
    case "failure": handleFailure()
    default: handleOther()
}
```

```ds expected
switch (status) {
    case "success":
        handleSuccess();
    case "failure":
        handleFailure();
    default:
        handleOther();
}
```
