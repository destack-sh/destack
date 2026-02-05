# Parameter

Parameter binding mutability depends on the language.

## Destack

### parameter bindings default to mutable in destack

> Destack parameters are mutable by default.

```ds
function bump(x: number): void {
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
