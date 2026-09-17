---
title: Newtypes
description: Proper nominality and newtypes.
---

# Newtypes

In TS land, we don't really have nominality of type aliases, so we're forced to use branding hacks with unique symbols to express nominal types:

```ts
const BrandTypeId: unique symbol = Symbol.for("effect/Brand")

type ProductId = number & {
  readonly [BrandTypeId]: {
    readonly ProductId: "ProductId" // unique identifier for ProductId
  }
}
```

TS++ adds proper newtypes with the `newtype` keyword, which functions much like `type`, except that the alias is nominal and must be explicitly cast into and out of:

```ds
newtype UserId = string;
const userId = UserId("123");
```

```ds
newtype UserId = private string; // can only be constructed within file
```
