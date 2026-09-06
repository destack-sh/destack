---
title: Newtypes
description: Proper nominality and newtypes.
---

# Newtypes

- usually use symbol branding in TS, which is kinda icky

```ts
const BrandTypeId: unique symbol = Symbol.for("effect/Brand")

type ProductId = number & {
  readonly [BrandTypeId]: {
    readonly ProductId: "ProductId" // unique identifier for ProductId
  }
}
```

- proper nominality and newtypes
- newtype, newtype interfaces (traits)

```ds
newtype UserId = string;
const userId = UserId("123");
```

```ds
newtype UserId = private string; // can only be constructed within file
```
