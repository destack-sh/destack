# Advanced Type Formatting

Tests for complex type expressions and type operators.

## Function Types

### function type alias

Function types keep parameter and return type spacing.

```ds
type Predicate = (value: string) => boolean
```

```ds expected
type Predicate = (value: string) => boolean;
```

### function type predicate return

Function type returns can narrow their parameter type.

```ds
type Is<T> = (value: any) => value is T
```

```ds expected
type Is<T> = (value: any) => value is T;
```

### function type predicate in conditional

Predicate function types stay parenthesized in conditional type constraints.

```ds
type Guarded<Actual> = Actual extends (value: any, ...args: any[]) => value is infer T ? T : never
```

```ds expected
type Guarded<Actual> = Actual extends ((value: any, ...args: any[]) => value is infer T)
    ? T
    : never;
```

### function type predicate comments

Comments before and after `is` stay inside the predicate return.

```ds
type Guard<T> = (value: unknown) => value /* value */ is /* type */ T
```

```ds expected
type Guard<T> = (value: unknown) => value /* value */ is /* type */ T;
```

### function type predicate comments in conditional

Predicate comments survive when the function type is the conditional constraint.

```ds
type PickGuard<Actual> = Actual extends (value: unknown) => value /* value */ is infer T ? T : never
```

```ds expected
type PickGuard<Actual> = Actual extends ((value: unknown) => value /* value */ is infer T)
    ? T
    : never;
```

## Indexed Access and Queries

### bracket tuple type

Bracket tuple types keep bracket syntax.

```ts:main.ts
type Pair = [T, boolean]
```

```ts expected
type Pair = [T, boolean];
```

### bracket tuple union

Empty and singleton bracket tuple types keep bracket syntax in unions.

```ts:main.ts
type Next<TNext> = [] | [TNext]
```

```ts expected
type Next<TNext> = [] | [TNext];
```

### bracket tuple rest element

Bracket tuple rest elements keep array suffixes on the rest type.

```ts:main.ts
type Requirements = [...PlatformCapability[]]
```

```ts expected
type Requirements = [...PlatformCapability[]];
```

### bracket tuple labeled rest payload

Labeled tuple rest elements keep the spread marker before the label.

```ts:main.ts
type RedisArgs = [...keys: RedisClient.KeyLike[], withscores: "WITHSCORES"]
```

```ts expected
type RedisArgs = [...keys: RedisClient.KeyLike[], withscores: "WITHSCORES"];
```

### bracket tuple optional label

Optional labeled tuple elements keep `?` on the label.

```ts:main.ts
type UpgradeOptions<WebSocketData> = [options?: {data?: undefined}, options: {data: WebSocketData}]
```

```ts expected
type UpgradeOptions<WebSocketData> = [
    options?: { data?: undefined },
    options: { data: WebSocketData },
];
```

### conditional bracket tuple optional label

Conditional tuple branches keep optional labels parseable after formatting.

```ts:main.ts
type UpgradeOptions<WebSocketData> = [WebSocketData] extends [undefined] ? [options?: {data?: undefined}] : [options: {data: WebSocketData}]
```

```ts expected
type UpgradeOptions<WebSocketData> = [WebSocketData] extends [undefined]
    ? [options?: { data?: undefined }]
    : [options: { data: WebSocketData }];
```

### indexed access type

Indexed access types keep brackets tight.

```ds
type Name = User["name"]
```

```ds expected
type Name = User["name"];
```

### type query

Type queries keep a space after `typeof`.

```ds
type Result = typeof someValue
```

```ds expected
type Result = typeof someValue;
```

### type import expression

Type import expressions keep the module target separate from additional arguments.

```ts:main.ts
type BlobRef = import("buffer").Blob
type JsonRef = import("vite", {with: {"resolution-mode": "import"}}).Plugin
```

```ts expected
type BlobRef = import("buffer").Blob;
type JsonRef = import("vite", { with: { "resolution-mode": "import" } }).Plugin;
```

## Conditional Types

### conditional with union branches

Conditional types format with spaces around `?` and `:`.

```ds
type Maybe<T> = T extends string ? T | null : T
```

```ds expected
type Maybe<T> = T extends string ? T | null : T;
```

### conditional branch comments

Conditional type branch comments stay attached to their original branch boundaries.

```ds
type Result<T> = T extends string // test-line
  ? // then-line
    StringValue
  : // else-line
    OtherValue
```

```ds expected
type Result<T> = T extends string // test-line
    ? // then-line
      StringValue
    : // else-line
      OtherValue;
```

## Type Template Literals

### typescript template literal type keeps assignment readable

Template literal types stay inline with `=` at wider line widths.

```ts:main.ts
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```ts expected
type templateLiteralType = `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""}`;
```

### typescript template literal type breaks after assignment at narrower widths

Template literal types break after `=` when the configured line width is narrower.

```ts:main.ts line-width=80
type templateLiteralType = `${
  TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
    ? "_"
    : ""
}`;
```

```ts expected
type templateLiteralType =
    `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}`;
```

### typescript template literal type with nested conditionals

Nested template literal types break with readable indentation.

```ts:main.ts line-width=80
type CamelToSnakeCase<TCamelCaseString extends string> =
  TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
    ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
        ? "_"
        : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
    : TCamelCaseString
```

```ts expected
type CamelToSnakeCase<TCamelCaseString extends string> =
    TCamelCaseString extends `${infer TStringConvertedSoFar}${infer TStringYetToConvert}`
        ? `${TStringConvertedSoFar extends Capitalize<TStringConvertedSoFar>
              ? "_"
              : ""}${Lowercase<TStringConvertedSoFar>}${CamelToSnakeCase<TStringYetToConvert>}`
        : TCamelCaseString;
```

## Type Operators

### keyof typeof chain

Combined `keyof` and `typeof` stays inline.

```ds
type Keys = keyof typeof values
```

```ds expected
type Keys = keyof typeof values;
```

## Ownership Types

### borrowed reference type

Borrowed references keep the `&` operator tight to the type.

```ds
type Borrowed = &Buffer
```

```ds expected
type Borrowed = &Buffer;
```

### readonly borrowed reference type

Readonly borrows keep `&readonly` tight.

```ds
type Borrowed = &readonly Buffer
```

```ds expected
type Borrowed = &readonly Buffer;
```

### owned reference type

Owned references keep the `^` operator tight.

```ds
type Owned = ^Result
```

```ds expected
type Owned = ^Result;
```

### ownership type comments

Comments after ownership operators group the target type.

```ds
type Handles = (& /* borrowed */ Buffer, ^ /* owned */ Result, * /* pointer */ Raw)
```

```ds expected
type Handles = (&(/* borrowed */ Buffer), ^(/* owned */ Result), *(/* pointer */ Raw));
```

### readonly ownership type comments

Comments after readonly ownership prefixes group the target type.

```ds
type Handles = (&readonly /* borrowed */ Buffer, *readonly /* pointer */ Raw)
```

```ds expected
type Handles = (&readonly (/* borrowed */ Buffer), *readonly (/* pointer */ Raw));
```

## Tuple Types

### tuple type alias

Tuple types keep parentheses and commas.

```ds
type Point = (int32, int32)
```

```ds expected
type Point = (int32, int32);
```

### nested conditional type

Nested conditional types preserve parentheses.

```ts:main.ts
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3
```

```ts expected
type Nested<T> = T extends string ? (T extends "a" ? 1 : 2) : 3;
```

## Intersections and Unions

### union inside intersection uses parentheses

Unions inside intersections are parenthesized.

```ds
type Combined = A & (B | C)
```

```ds expected
type Combined = A & (B | C);
```

### intersection inside union uses parentheses

Intersections inside unions are parenthesized.

```ds
type Combined = A | (B & C)
```

```ds expected
type Combined = A | (B & C);
```

## Mapped Types (TypeScript)

### mapped type keeps bracket spacing

Mapped types include spaces when bracket spacing is enabled.

```ts:main.ts
export type Bar<T> = {[P in keyof T]: string}
```

```ts expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type modifiers

Mapped type modifiers keep their prefixes and suffixes.

```ts:main.ts
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] }
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] }
```

```ts expected
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] };
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] };
```

### mapped type key remap

Mapped type key remaps keep `as` spacing.

```ts:main.ts
type EventHandlers<T> = { [K in keyof T as `on${Capitalize<K & string>}`]?: T[K] }
```

```ts expected
type EventHandlers<T> = { [K in keyof T as `on${Capitalize<K & string>}`]?: T[K] };
```

### mapped type separator comments

Mapped type comments format at remap and value boundaries.

```ds
type Flags<T> = {
  [K in keyof T as /* remap */ `can${Capitalize<K & string>}`]: /* value */ boolean
}

type Values<T> = {
  [K in keyof T]: // value-line
    boolean
}
```

```ds expected
type Flags<T> = {
    [K in keyof T as /* remap */ `can${Capitalize<K & string>}`]: /* value */ boolean;
};

type Values<T> = {
    [K in keyof T]: boolean; // value-line
};
```

### exported mapped type stays inline

Short exported mapped types stay inline under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
export type Bar<T> = {[P in keyof T]: string}
```

```ts expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type key remapping keeps remap expressions

Mapped types with `as` remaps preserve their remap expressions under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
type MappedTypeWithNewKeys<T> = {
  [K in keyof T as NewKeyType]: T[K]
};

type RemoveKindField<T> = {
  [K in keyof T as Exclude<K, "kind">]: T[K]
};

type PickByValueType<T, U> = {
  [K in keyof T as T[K] extends U ? K : never]: T[K]
};
```

```ts expected
type MappedTypeWithNewKeys<T> = {
  [K in keyof T as NewKeyType]: T[K];
};

type RemoveKindField<T> = {
  [K in keyof T as Exclude<K, "kind">]: T[K];
};

type PickByValueType<T, U> = {
  [K in keyof T as T[K] extends U ? K : never]: T[K];
};
```

## Conditional Types (TypeScript)

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

## Type Parameter Constraints And Defaults

### long type parameter constraints and defaults break cleanly

Long generic constraints and defaults break cleanly under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
export type OuterType1<
  LongerLongerLongerLongerInnerType extends LongerLongerLongerLongerOtherType<OneMoreType>
> = { a: 1 };
export type OuterType12<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerOtherType<OneMoreType>
> = { a: 1 };

export type OuterType2<
  LongerLongerLongerLongerInnerType extends LongerLongerLongerLongerLongerLongerLongerLongerOtherType
> = { a: 1 };
export type OuterType22<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerLongerLongerLongerLongerOtherType
> = { a: 1 };

export type OuterType3<
  LongerLongerLongerLongerInnerType extends LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType
> = { a: 1 };
export type OuterType32<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType
> = { a: 1 };

export type OuterType4<
  LongerLongerLongerLongerInnerType extends
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType
> = { a: 1 };
export type OuterType42<
  LongerLongerLongerLongerInnerType =
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType
> = { a: 1 };
```

```ts expected
export type OuterType1<
  LongerLongerLongerLongerInnerType extends
    LongerLongerLongerLongerOtherType<OneMoreType>,
> = { a: 1 };
export type OuterType12<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerOtherType<OneMoreType>,
> = { a: 1 };

export type OuterType2<
  LongerLongerLongerLongerInnerType extends
    LongerLongerLongerLongerLongerLongerLongerLongerOtherType,
> = { a: 1 };
export type OuterType22<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerLongerLongerLongerLongerOtherType,
> = { a: 1 };

export type OuterType3<
  LongerLongerLongerLongerInnerType extends
    LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType,
> = { a: 1 };
export type OuterType32<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType,
> = { a: 1 };

export type OuterType4<
  LongerLongerLongerLongerInnerType extends
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType,
> = { a: 1 };
export type OuterType42<
  LongerLongerLongerLongerInnerType =
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType,
> = { a: 1 };
```
