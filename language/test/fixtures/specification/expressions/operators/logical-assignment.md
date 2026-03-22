# Logical Assignment

## logical or assignment

### logical or assignment accepts assignable values

> Logical or assignment uses the same assignment rules.

```ts
let value: string | undefined = undefined;

value ||= "fallback";
value satisfies string | undefined;
```

### logical or assignment rejects incompatible values

> Logical or assignment rejects incompatible right hand side values.

```ts
let value: string = "ok";

value ||= 1;
```

- type string | 1 is not assignable to type string | undefined

## logical and assignment

### logical and assignment preserves assignable values

> Logical and assignment uses the same assignment rules.

```ts
type Value = { ok: true };

declare const fallback: Value;
let value: Value | undefined = fallback;

value &&= fallback;
value satisfies Value | undefined;
```

### logical and assignment rejects incompatible values

> Logical and assignment rejects incompatible right hand side values.

```ts
let value: string | undefined = "ok";

value &&= 1;
```

- type string | 1 is not assignable to type string | undefined

## nullish assignment

### nullish assignment preserves assignable values

> Nullish assignment uses the same assignment compatibility rules.

```ts
let value: string | undefined = undefined;

value ??= "fallback";
value satisfies string | undefined;
```

### nullish assignment rejects incompatible values

> Nullish assignment rejects incompatible right hand side values.

```ts
let value: string | undefined = undefined;

value ??= 1;
```

- type string | 1 is not assignable to type string | undefined