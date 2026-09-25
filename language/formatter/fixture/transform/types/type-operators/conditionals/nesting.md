# Nested Conditional Types

## Nested Conditional Types

### conditional type with nested parentheses

Conditional types keep parentheses and break cleanly.

```tspp:main.tspp line-width=80
type IsUnion<T> = (
  Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
  ? false
  : true
```

```tspp expected
type IsUnion<T> = (
    Testtttttttttttttttttttttttttttttttttt extends any ? false : never
) extends false
    ? false
    : true;
```

### conditional type with infer

Infer types stay inline with the `extends` clause.

```tspp:main.tspp
type Unpacked<T> = T extends (infer U)[] ? U : T
```

```tspp expected
type Unpacked<T> = T extends (infer U)[] ? U : T;
```

### conditional type with constrained infer

Constrained `infer` clauses stay attached to the `extends` boundary under non-default formatter options.

```tspp:main.tspp indent-width=2 line-width=80
type X3<T> = T extends (infer U extends number,) ? MustBeNumber<U> : never;
type X4<T> = T extends (infer U extends number, infer U extends number) ? MustBeNumber<U> : never;
type X5<T> = T extends (infer U extends number, infer U) ? MustBeNumber<U> : never;
type X6<T> = T extends (infer U, infer U extends number) ? MustBeNumber<U> : never;
type X7<T> = T extends (infer U extends string, infer U extends number) ? U : never;
type X8<U, T> = T extends infer U extends number ? U : T;
type X9<U, T> = T extends (infer U extends number ? U : T) ? U : T;
type X10<T> = T extends (infer U extends number) | { a: infer U extends number } ? U : never
type X11<T> = T extends (infer U extends number) & { a: infer U extends number } ? U : never
```

```tspp expected
type X3<T> = T extends (infer U extends number,) ? MustBeNumber<U> : never;
type X4<T> = T extends (infer U extends number, infer U extends number)
  ? MustBeNumber<U>
  : never;
type X5<T> = T extends (infer U extends number, infer U)
  ? MustBeNumber<U>
  : never;
type X6<T> = T extends (infer U, infer U extends number)
  ? MustBeNumber<U>
  : never;
type X7<T> = T extends (infer U extends string, infer U extends number)
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
