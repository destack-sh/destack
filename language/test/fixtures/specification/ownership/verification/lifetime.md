# Lifetime Verification

## escaping borrows

### returning a reference to a local is rejected

> References to locals cannot escape their scope.

```ds native=true
struct Data {
    value: int32,
}

struct Container {
    data: Data,
}

function makeDataRef(): &Data {
    let container = ^Container { data: Data { value: 1 } };
    &container.data
}
```

- contains: cannot return reference to local

## lifetime annotations

### return must match lifetime annotation

> Borrowed returns must satisfy the declared lifetime.

```ds native=true
@lifetime("a")
function pick(a: &int32, b: &int32): &int32 {
    b
}
```

- contains: return borrows from b not covered by lifetime annotation
