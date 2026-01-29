# Using Statements

Tests for using declarations and await using formatting.

## Basic Using

### using declaration

Using declarations keep spacing around `=`.

```ds
using resource=open()
```

```ds expected
using resource = open();
```

### await using declaration

Await using preserves the `await` keyword.

```ds
await using conn = open()
```

```ds expected
await using conn = open();
```
