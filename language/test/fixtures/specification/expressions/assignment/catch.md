# Catch Bindings

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
    if (typeof e === "string") {
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

### catch annotations reject concrete types

Concrete catch annotations are not filters.

```ds
declare function fail(): Result<void, string>;

try {
    fail()?;
} catch (e: string) {
    e = "fix";
}
```

- contains: catch type annotations must be unknown
