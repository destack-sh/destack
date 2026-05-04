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

> Catch annotations accept `unknown`.

```ts:main.ts
try {
    throw "boom";
} catch (e: unknown) {
    e = "fix";
}
```

### catch annotations reject concrete types

> Catch annotations only allow `unknown`.

```ts:main.ts
try {
    throw "boom";
} catch (e: string) {
    e = "fix";
}
```

- contains: catch type annotations must be unknown
