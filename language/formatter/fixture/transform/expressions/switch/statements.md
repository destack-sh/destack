# Switch Statements

Switch statement fixtures cover case bodies, default bodies, and statement tails.

## Cases

### switch expression

Switch cases use case/colon syntax.

```tspp
switch (x) {
    case 1: "one"
    case 2: "two"
}
```

```tspp expected
switch (x) {
    case 1:
        "one";
    case 2:
        "two";
}
```

### switch with default case

Default cases use the default keyword.

```tspp
switch (value) {
    case 1: "one"
    default: "other"
}
```

```tspp expected
switch (value) {
    case 1:
        "one";
    default:
        "other";
}
```

### switch with string patterns

Switch works with string literal patterns.

```tspp
switch (type) {
    case "static": handleStatic()
    case "dynamic": handleDynamic()
    default: handleDefault()
}
```

```tspp expected
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

```tspp
switch (type) {
    case "static": label = "static";
    default: label = "other";
}
```

```tspp expected
switch (type) {
    case "static":
        label = "static";
    default:
        label = "other";
}
```
