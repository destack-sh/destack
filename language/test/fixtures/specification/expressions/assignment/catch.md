# Catch

Catch binding mutability depends on the language.

## Destack

### catch bindings default to immutable in destack

> Destack catch bindings are immutable unless marked mutable.

```ds
const value = try {
    1
} catch e {
    e = 2;
    0
};
value satisfies int;
```

- contains: immutable binding

### catch bindings allow explicit mutability in destack

> Destack catch bindings can be marked mutable with `mut`.

```ds
const value = try {
    1
} catch mut e {
    e = 2;
    0
};
value satisfies int;
```

## TypeScript

### catch bindings default to mutable in typescript

> TypeScript catch bindings are mutable by default.

```ts:main.ts
try {
    throw "boom";
} catch (e) {
    e = "fix";
}
```
