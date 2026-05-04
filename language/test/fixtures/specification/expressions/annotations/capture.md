# Capture Annotations

`@capture` is a compiler-known rewrite annotation for closure capture policy.

## policy

### capture applies to named functions

> Function declarations can declare their closure capture policy.

```ds
@capture("byValue")
function make() {
    return () => 1;
}
```

### capture applies to returned lambdas

> A returned lambda can carry its own capture policy.

```ds
function make() {
    @capture("byReference")
    return () => 1;
}
```

### capture accepts byMove

> `byMove` is a valid capture policy.

```ds
@capture("byMove")
function make() {
    return () => 1;
}
```

### capture can configure this

> Capture rules can name `this`.

```ds
class Counter {
    value: number = 0;

    make(): () => number {
        @capture({ this: "byValue" })
        return () => this.value;
    }
}
```

## validation

### capture rejects unsupported targets

> `@capture` only supports function-like targets.

```ds
@capture("byValue")
const value = 1;
```

- contains: capture annotation is only supported on function declarations

### capture rejects unknown policies

> Capture policies are closed string literals.

```ds
@capture("maybe")
function make() {
    return () => 1;
}
```

- capture policy must be "byValue", "byReference", or "byMove"

### capture rejects dynamic policies

> Capture policy arguments must be statically visible.

```ds
const kind = "byValue";

@capture(kind)
function make() {
    return () => 1;
}
```

- contains: capture annotation argument must be a string or object literal

### capture rejects non-string rule values

> Capture rule values must be string literals.

```ds
@capture({ a: 1 })
function make() {
    const a = 1;

    return () => a;
}
```

- invalid well-known annotation: capture annotation values must be string literals
