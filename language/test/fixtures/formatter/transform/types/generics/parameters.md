# Type Parameter Constraints and Defaults

## Type Parameter Constraints and Defaults

### long type parameter constraints and defaults break cleanly

Long generic constraints and defaults break cleanly under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80
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
