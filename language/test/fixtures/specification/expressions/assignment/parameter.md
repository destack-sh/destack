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

### parameter assignments still enforce parameter types in destack

> Mutable parameter bindings still enforce declared parameter types.

```ds
function bump(x: number): void {
    x = "no";
}
```

- not assignable

### parameter assignments still enforce parameter types in typescript

> TypeScript mutable parameter bindings still enforce declared parameter types.

```ts:main.ts
function bump(x: number): void {
    x = "no";
}
```

- not assignable

### parameter bindings allow compound assignment in destack

> Destack parameter bindings support compound assignment operators.

```ds
function bump(x: number): number {
    x += 2;
    x
}
```

### parameter bindings allow compound assignment in typescript

> TypeScript parameter bindings support compound assignment operators.

```ts:main.ts
function bump(x: number): number {
    x += 2;
    return x;
}
```
