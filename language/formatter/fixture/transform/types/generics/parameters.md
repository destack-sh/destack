# Type Parameter Constraints and Defaults

## Type Parameter Constraints and Defaults

### const and variance modifiers

Type parameter modifiers keep their declaration order.

```tspp
function id< const T , U >(value: T): T { return value }
class Box< out T , const U > {
    method< const V , in W >(value: V): V { return value }
}
```

```tspp expected
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

```tspp:main.tspp indent-width=2 line-width=80
export type OuterType1<
  LongerLongerLongerLongerInnerType: LongerLongerLongerLongerOtherType<OneMoreType>
> = { a: 1 };
export type OuterType12<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerOtherType<OneMoreType>
> = { a: 1 };

export type OuterType2<
  LongerLongerLongerLongerInnerType: LongerLongerLongerLongerLongerLongerLongerLongerOtherType
> = { a: 1 };
export type OuterType22<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerLongerLongerLongerLongerOtherType
> = { a: 1 };

export type OuterType3<
  LongerLongerLongerLongerInnerType: LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType
> = { a: 1 };
export type OuterType32<
  LongerLongerLongerLongerInnerType = LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType
> = { a: 1 };

export type OuterType4<
  LongerLongerLongerLongerInnerType:
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType
> = { a: 1 };
export type OuterType42<
  LongerLongerLongerLongerInnerType =
    | LongerLongerLongerLongerLongerLo
    | ngerLongerLongerOtherType
> = { a: 1 };
```

```tspp expected
export type OuterType1<
  LongerLongerLongerLongerInnerType:
    LongerLongerLongerLongerOtherType<OneMoreType>,
> = { a: 1 };
export type OuterType12<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerOtherType<OneMoreType>,
> = { a: 1 };

export type OuterType2<
  LongerLongerLongerLongerInnerType:
    LongerLongerLongerLongerLongerLongerLongerLongerOtherType,
> = { a: 1 };
export type OuterType22<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerLongerLongerLongerLongerOtherType,
> = { a: 1 };

export type OuterType3<
  LongerLongerLongerLongerInnerType:
    LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType,
> = { a: 1 };
export type OuterType32<
  LongerLongerLongerLongerInnerType =
    LongerLongerLongerLongerLongerLo.ngerLongerLongerOtherType,
> = { a: 1 };

export type OuterType4<
  LongerLongerLongerLongerInnerType:
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

```tspp
type Callback< ...Parameters , Return > = (...parameters: Parameters) => Return
```

```tspp expected
type Callback<...Parameters, Return> = (...parameters: Parameters) => Return;
```

### variadic value parameters

Variadic value parameters keep the `const` marker before the spread marker.

```tspp
function tensor< const ...Shape : readonly usize[] >(value: Tensor< ...Shape >): void {}
```

```tspp expected
function tensor<const ...Shape: readonly usize[]>(value: Tensor<...Shape>): void {}
```
