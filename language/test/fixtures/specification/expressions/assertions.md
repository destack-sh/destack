# Assertion Expressions

## type assertions

### type assertions preserve the asserted type

> `as` assertions apply the target type.

```ts
const value = "ok" as string;
value satisfies string;
```

### type assertions can narrow unions

> Assertions can select a narrower union member.

```ts
const value = ("ok" as string | number) as string;
value satisfies string;
```

## non-null assertions

### non-null assertions remove null and undefined

> Non-null assertions remove nullish types from the expression.

```ts
declare const value: string | null | undefined;

const out = value!;
out satisfies string;
```

### non-null assertions preserve non nullish types

> Non-null assertions keep the existing type when it is non nullish.

```ts
const value: string = "ok";

const out = value!;
out satisfies string;
```
