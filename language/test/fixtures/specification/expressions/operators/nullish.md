# Nullish Coalescing

`??` and `??=` select a fallback only for nullish values.

## nullish coalescing

### nullish coalescing selects fallback for null

> Nullish coalescing selects the right side when the left is null.

```ts
declare const value: string | null;

const out = value ?? "fallback";
out satisfies string;
```

### nullish coalescing removes undefined from unions

> Nullish coalescing removes undefined from the left side.

```ts
declare const value: string | undefined;

const out = value ?? "fallback";
out satisfies string;
```

### nullish coalescing preserves non nullish unions

> Non nullish union members remain in the result.

```ts
declare const value: string | number | null;

const out = value ?? 1;
out satisfies string | number;
```

## nullish assignment

### nullish assignment preserves the non nullish type

> Nullish assignment keeps the assigned type when the value is nullish.

```ts
let value: string | null = null;

value ??= "fallback";
value satisfies string;
```

### nullish assignment rejects incompatible values

> Nullish assignment uses standard assignment checks.

```ts
let value: string | null = null;

value ??= 1;
```

- type string | 1 is not assignable to type string | null
