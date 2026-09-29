---
title: Newtypes
description: Nominal names for existing types, at no runtime cost.
---

# Newtypes

In TS land, type aliases are structural, so a `UserId` and an `OrderId` that are both strings mix freely.
The usual workaround is branding with a unique symbol:

```ts
const BrandTypeId: unique symbol = Symbol.for("effect/Brand");

type ProductId = number & {
    readonly [BrandTypeId]: {
        readonly ProductId: "ProductId"; // unique identifier for ProductId
    };
};
```

TS++ has proper newtypes instead.
A `newtype` works much like `type`, except the name is nominal: values convert into and out of it explicitly, and at runtime it is just its backing type.

```tspp
newtype UserId = string;
newtype OrderId = string;

const user = UserId("u-123");
const order: OrderId = user;
//                     ^^^^ error[not-assignable]: type 'UserId' is not assignable to type 'OrderId'
```

## Converting

`as` unwraps or wraps exactly one layer, so getting from one newtype to another goes through the backing type on purpose.

```tspp
newtype UserId = int64;
newtype AccountId = int64;

function account(user: UserId): AccountId {
    user as int64 as AccountId
}
```

When the expected type is already known, `_(...)` constructs it without repeating the name.

```tspp
newtype UserId = int64;

const id: UserId = _(42);
```

## Members

A newtype reads the members of its backing type, and extensions add its own.

```tspp
newtype Email = string;

extension of Email {
    domain(): string {
        this.split("@")[1]
    }
}

const email = Email("florian@destack.sh");
email.length // => 18
email.domain() // => "destack.sh"
```

## Private Backing

Writing `private` before the backing type keeps construction and unwrapping inside the declaring module, so the module decides what a valid value is.

```tspp src/token.tspp
export newtype Token = private string;

export function issue(user: string): Token {
    Token(`token:${user}`)
}
```

```tspp src/main.tspp
import { Token, issue } from "./token.tspp";

const token = issue("florian");
const text = token as string;
//           ^^^^^ error[inaccessible-newtype-backing]: the backing of 'Token' is private
```
