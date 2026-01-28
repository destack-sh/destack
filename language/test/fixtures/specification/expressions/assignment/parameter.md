# Parameter

Parameter binding mutability depends on the language.

## Destack

### parameter bindings default to immutable in destack

> Destack parameters are immutable unless marked mutable.

```ds
function bump(x: number): void {
    x = 2;
}
```

- contains: immutable binding

### parameter bindings allow explicit mutability in destack

> Destack parameters can be marked mutable with `mut`.

```ds
function bump(mut x: number): void {
    x = 2;
    x satisfies number;
}
```

## TypeScript

### parameter bindings default to mutable in typescript

> TypeScript parameters are mutable by default.

```ts:main.ts
function bump(x: number): void {
    x = 2;
}
```
