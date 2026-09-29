---
title: Type Operators
description: Type inference and static term evaluation are one and the same.
---

# Type Operators

## Keys

```tspp
type User = { name: string; age: int32 };
type Keys = keyof User;

declare const key: Keys;
```

## Indexed Access

```tspp
type User = { name: string; age: int32 };
type Name = User["name"];

declare const name: Name;
```

## Intersections

```tspp
type Named = { name: string };
type Aged = { age: int32 };
type Person = Named & Aged;

declare const person: Person;
const name = person.name;
const age = person.age;
```

## Conditional Types

```tspp
type Select<T> = T extends string ? "yes" : "no";
type Text = Select<string>;
type Number = Select<int32>;

declare const text: Text;
declare const number: Number;
```

## Inference

```tspp
type Box<T> = { value: T };
type Unbox<T> = T extends Box<infer U> ? U : never;
type Value = Unbox<Box<"ready">>;

declare const value: Value;
```

## Mapped Types

- TS algebra is actually great for doing light metaprogramming
- want to preserve as much of it as possible
- thankfully, it's actually mostly sound already
- especially once we agree that structural types are exact / fixed, and interfaces are dynamic

```tspp
export type Record<K: PropertyKey, V> = {
    [P in K]: V;
};
```

## Utility Types

- utility types!
- mapped types
- Pick, Readonly, ...
- ThisParameterType
- .. all the other utility types
