# Catch

Catch binding mutability depends on the language.

## Destack

### catch bindings default to mutable in destack

> Destack catch bindings are mutable by default.

```ds
const value = try {
    1
} catch e {
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

### catch bindings remain mutable with useUnknownInCatchVariables

> Catch binding mutability is independent from unknown catch variable typing.

```json:dsconfig.json
{ "compilerOptions": { "allowTs": true, "checkTs": true, "useUnknownInCatchVariables": true } }
```

```ts:main.ts
try {
    throw "boom";
} catch (e) {
    e = { message: "fix" };
}
```

### catch bindings allow reassignment after narrowing checks

> Catch bindings can still be reassigned after control-flow narrowing checks.

```ts:main.ts
try {
    throw "boom";
} catch (e) {
    if (typeof e === "string") {
        e = e.toUpperCase();
    }
}
```

### catch annotations reject concrete types in typescript

> TypeScript catch annotations only allow `any` or `unknown`.

```ts:main.ts
try {
    throw "boom";
} catch (e: string) {
    e = "fix";
}
```

- contains: catch type annotations must be 'any' or 'unknown'

### catch annotations allow unknown in typescript

> TypeScript catch annotations accept `unknown`.

```ts:main.ts
try {
    throw "boom";
} catch (e: unknown) {
    e = "fix";
}
```
