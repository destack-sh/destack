# Catch Bindings

Catch bindings are ordinary bindings scoped to their handler.

## reassignment

### catch bindings are mutable

Catch bindings can be reassigned.

```ds
declare function fail(): Result<void, string>;

try {
    fail()?;
} catch (e) {
    e = "fix";
}
```

### catch bindings stay mutable after narrowing

Narrowing does not make a catch binding immutable.

```ds
declare function fail(): Result<void, string>;

try {
    fail()?;
} catch (e) {
    if (e is string) {
        e = e.toUpperCase();
    }
}
```

## annotations

### catch annotations allow unknown

Catch bindings can be annotated as `unknown`.

```ds
declare function fail(): Result<void, string>;

try {
    fail()?;
} catch (e: unknown) {
    e = "fix";
}
```

### catch annotations can state the propagated failure type

Catch annotations are ordinary binding annotations.

```ds
declare function fail(): Result<void, string>;

try {
    fail()?;
} catch (e: string) {
    e = "fix";
}
```

### catch annotations reject incompatible failures

Catch annotations are not filters.
The propagated failure must be assignable to the annotation.

```ds
declare function fail(): Result<void, int32>;

try {
    fail()?;
} catch (e: string) {
    e = "fix";
}
```

- contains: not assignable
