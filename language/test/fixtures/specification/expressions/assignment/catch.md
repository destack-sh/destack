# Catch Bindings

## reassignment

### catch bindings are mutable

> Catch bindings can be reassigned.

```ds
try {
    throw "boom";
} catch (e) {
    e = "fix";
}
```

### catch bindings remain mutable after narrowing

> Catch bindings can be reassigned after control-flow narrowing checks.

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

`unknown` is the only allowed catch annotation.

```ts:main.ts
try {
    throw "boom";
} catch (e: unknown) {
    e = "fix";
}
```

### catch annotations reject concrete types

Concrete catch annotations are not filters.

```ts:main.ts
try {
    throw "boom";
} catch (e: string) {
    e = "fix";
}
```

- contains: catch type annotations must be unknown
