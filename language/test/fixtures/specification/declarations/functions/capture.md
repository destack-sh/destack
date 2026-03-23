# Capture Decorators

Tests for capture decorator validation and basic usage.

## Valid Decorators

### capture allows named functions

@capture should be accepted on named functions.

```ds
@capture("byValue")
function make() {
  return () => 1;
}
```

### capture allows lambdas

@capture should be accepted on lambda declarations.

```ds
function make() {
  @capture("byReference")
  return () => 1;
}
```

### capture allows byMove

@capture should accept the byMove policy.

```ds
@capture("byMove")
function make() {
  return () => 1;
}
```

### capture allows this overrides

@capture should allow a `this` entry in the rules map.

```ds
class Counter {
  value: number = 0;

  make(): () => number {
    @capture({ this: "byValue" })
    return () => this.value;
  }
}
```

## Invalid Decorators

### capture rejects non function targets

@capture should reject non function declarations.

```ds
@capture("byValue")
const value = 1;
```

- contains: capture decorator is only supported on function declarations

### capture rejects unknown policy strings

@capture should reject unknown policy strings.

```ds
@capture("maybe")
function make() {
  return () => 1;
}
```

- capture policy must be "byValue", "byReference", or "byMove"

### capture rejects non literal arguments

@capture should reject non literal arguments.

```ds
const kind = "byValue";
@capture(kind)
function make() {
  return () => 1;
}
```

- contains: capture decorator argument must be a string or object literal

### capture rejects non string rule values

@capture should reject non string rule values.

```ds
@capture({ a: 1 })
function make() {
  const a = 1;
  return () => a;
}
```

- invalid well-known decorator: capture decorator values must be string literals