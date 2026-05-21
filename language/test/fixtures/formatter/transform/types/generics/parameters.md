# Type Parameter Constraints and Defaults

## Type Parameter Constraints and Defaults

### const and variance modifiers

Type parameter modifiers keep their declaration order.

```ts
function id< const T , U >(value: T): T { return value }
class Box< out T , const U > {
    method< const V , in W >(value: V): V { return value }
}
```

```ts expected
function id<const T, U>(value: T): T {
    return value;
}
class Box<out T, const U> {
    method<const V, in W>(value: V): V {
        return value;
    }
}
```

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

### variadic type parameters

Variadic type parameters keep the spread marker attached to the name.

```ds
type Callback< ...Parameters , Return > = (...parameters: Parameters) => Return
```

```ds expected
type Callback<...Parameters, Return> = (...parameters: Parameters) => Return;
```

### variadic value parameters

Variadic value parameters keep the `comptime` marker before the spread marker.

```ds
function tensor< comptime ...Shape : readonly usize[] >(value: Tensor< ...Shape >): void {}
```

```ds expected
function tensor<comptime ...Shape: readonly usize[]>(value: Tensor<...Shape>): void {}
```
