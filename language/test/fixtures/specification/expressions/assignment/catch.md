# Catch Bindings

## reassignment

### catch bindings are mutable

Catch bindings can be reassigned.

```ds
try {
    throw "boom";
} catch (e) {
    e = "fix";
}
```

### catch bindings stay mutable after narrowing

Narrowing does not make a catch binding immutable.

```ds
try {
    throw "boom";
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
try {
    throw "boom";
} catch (e: unknown) {
    e = "fix";
}
```

### catch annotations reject concrete types

Concrete catch annotations are not filters.

```ds
try {
    throw "boom";
} catch (e: string) {
    e = "fix";
}
```

- contains: catch type annotations must be unknown
