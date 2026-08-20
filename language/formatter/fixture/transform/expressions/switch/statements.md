# Switch Statements

Switch statement fixtures cover case bodies, default bodies, and statement tails.

## Cases

### switch expression

Switch cases use case/colon syntax.

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

### switch assignment body

Assignment statements in switch cases stay direct.

```ds
switch (type) {
    case "static": label = "static";
    default: label = "other";
}
```

```ds expected
switch (type) {
    case "static":
        label = "static";
    default:
        label = "other";
}
```
