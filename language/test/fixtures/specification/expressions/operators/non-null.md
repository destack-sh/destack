# Non Null

Postfix `!` removes nullish members from an expression type.

## expression

### non-null removes null and undefined

> Postfix `!` removes nullish types from the expression.

```ts
declare const value: string | null | undefined;

const out = value!;
out satisfies string;
```

### non-null preserves non-nullish types

> Postfix `!` keeps the existing type when it is non-nullish.

```ts
const value: string = "ok";

const out = value!;
out satisfies string;
```

### non-null preserves unrelated union members

> Postfix `!` only removes nullish members from a union.

```ts
declare const value: string | number | null;

const out = value!;
out satisfies string | number;
```

### assertion chains preserve the final target

> Chained assertions use the final target type.

```ts
const value = ("ok" as unknown) as { length: number };
value.length satisfies number;
```
