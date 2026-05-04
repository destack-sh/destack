# Regions

## escaping borrows

### returning reference to local is rejected

> References to locals cannot escape their scope.

```ds native=true
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

function makeDataRef(): &Data {
    let container = ^Container { data: Data { value: 1 } };
    &container.data
}
```

- contains: cannot return reference to local

## region annotations

### return must match lifetime annotation

> Borrowed returns must satisfy the declared lifetime.

```ds native=true
@lifetime("a")
function pick(a: &int32, b: &int32): &int32 {
    b
}
```

- contains: return borrows from b not covered by lifetime annotation

### return matches lifetime annotation

> Borrowed returns are allowed when they match the declared lifetime.

```ds native=true
@lifetime("a")
function pick(a: &int32, b: &int32): &int32 {
    a
}
```

### return can borrow from any annotated parameter

> Multiple lifetimes allow returns to borrow from any listed parameter.

```ds native=true
@lifetime("a", "b")
function pick(a: &int32, b: &int32, flag: boolean): &int32 {
    if (flag) {
        a
    } else {
        b
    }
}
```

### static lifetime does not allow parameter borrows

> Static lifetimes cannot be satisfied by parameter borrows.

```ds native=true
@lifetime("static")
function pick(value: &int32): &int32 {
    value
}
```

- contains: return borrows from value not covered by lifetime annotation
