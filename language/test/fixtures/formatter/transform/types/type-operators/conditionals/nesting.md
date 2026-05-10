# Nested Conditional Types

## Nested Conditional Types

### conditional type with nested parentheses

Conditional types keep parentheses and break cleanly.

```ts:main.ts line-width=80
type IsUnion<T> = (
  Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
  ? false
  : true
```

```ts expected
type IsUnion<T> = (
    Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
    ? false
    : true;
```

### conditional type with infer

Infer types stay inline with the `extends` clause.

```ts:main.ts
type Unpacked<T> = T extends (infer U)[] ? U : T
```

```ts expected
type Unpacked<T> = T extends (infer U)[] ? U : T;
```

### conditional type with constrained infer

Constrained `infer` clauses stay attached to the `extends` boundary under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
type X3<T> = T extends [infer U extends number] ? MustBeNumber<U> : never;
type X4<T> = T extends [infer U extends number, infer U extends number] ? MustBeNumber<U> : never;
type X5<T> = T extends [infer U extends number, infer U] ? MustBeNumber<U> : never;
type X6<T> = T extends [infer U, infer U extends number] ? MustBeNumber<U> : never;
type X7<T> = T extends [infer U extends string, infer U extends number] ? U : never;
type X8<U, T> = T extends infer U extends number ? U : T;
type X9<U, T> = T extends (infer U extends number ? U : T) ? U : T;
type X10<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never
type X11<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never
```

```ts expected
type X3<T> = T extends [infer U extends number] ? MustBeNumber<U> : never;
type X4<T> = T extends [infer U extends number, infer U extends number]
  ? MustBeNumber<U>
  : never;
type X5<T> = T extends [infer U extends number, infer U]
  ? MustBeNumber<U>
  : never;
type X6<T> = T extends [infer U, infer U extends number]
  ? MustBeNumber<U>
  : never;
type X7<T> = T extends [infer U extends string, infer U extends number]
  ? U
  : never;
type X8<U, T> = T extends infer U extends number ? U : T;
type X9<U, T> = T extends (infer U extends number ? U : T) ? U : T;
type X10<T> = T extends (infer U extends number) | { a: infer U extends number }
  ? U
  : never;
type X11<T> = T extends (infer U extends number) & { a: infer U extends number }
  ? U
  : never;
```
