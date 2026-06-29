# Extension Declarations

## Extension Forms

### extension declaration

Extensions use `of` to specify the type being extended.

```ds
extension of  Vector2  { }
```

Empty extension bodies stay on one line with internal spacing.

```ds expected
extension of Vector2 {}
```

### extension with implements

Extensions can implement traits for the extended type.

```ds
extension of  Vector2  implements  Add < Vector2 >  { }
```

Generic type arguments have no internal spacing.

```ds expected
extension of Vector2 implements Add<Vector2> {}
```

### extension with multiline implements

Long implements lists break after the keyword and indent each implemented type.

```ds line-width=80
extension<T> of Deque<T> implements Index<number>, IndexSet<number, T>, Iterable<T>, Iterable<&readonly T>, Extend<T, "exclusive"> {
    index(index: number): T;
}
```

```ds expected
extension<T> of Deque<T> implements
    Index<number>,
    IndexSet<number, T>,
    Iterable<T>,
    Iterable<&readonly T>,
    Extend<T, "exclusive"> {
    index(index: number): T;
}
```

### extension with multiline target

Long extension targets break after `of` and keep the target indented under the header.

```ds line-width=100
extension<T, comptime Rank: int, F: TensorFormat, comptime ...Axes: ShardingAxis> of Tensor<T, Rank, F, Sharding<...Axes>> {
    get mesh(): Mesh;
}
```

```ds expected
extension<T, comptime Rank: int, F: TensorFormat, comptime ...Axes: ShardingAxis> of
    Tensor<T, Rank, F, Sharding<...Axes>> {
    get mesh(): Mesh;
}
```

### extension with multiline target and implements

Long extension targets break before the implemented trait list.

```ds line-width=80
extension<T: int | float, comptime Rank: int, F: TensorFormat, P: Placement> of Tensor<T, Rank, F, P> implements Add<Tensor<T, Rank, F, P>>, Subtract<Tensor<T, Rank, F, P>>, Multiply<Tensor<T, Rank, F, P>> {
    type Output = Tensor<T, Rank, F, P>;
}
```

```ds expected
extension<T: int | float, comptime Rank: int, F: TensorFormat, P: Placement> of
    Tensor<T, Rank, F, P>
    implements
        Add<Tensor<T, Rank, F, P>>,
        Subtract<Tensor<T, Rank, F, P>>,
        Multiply<Tensor<T, Rank, F, P>> {
    type Output = Tensor<T, Rank, F, P>;
}
```

### extension implemented type with where clause

Where clauses on implemented types stay attached to the implemented type.

```ds line-width=100
extension<T, comptime N: number, R: RangeBounds<usize>> of FixedArray<T, N> implements IndexSet<R, Slice<T>> where T: Copy {
    indexSet(&exclusive this, range: R, source: Slice<T>): void;
}
```

```ds expected
extension<T, comptime N: number, R: RangeBounds<usize>> of FixedArray<T, N> implements
    IndexSet<R, Slice<T>> where T: Copy {
    indexSet(&exclusive this, range: R, source: Slice<T>): void;
}
```

## Named Extensions

### named extension

Named extensions include the name before `of`.

```ds
extension  MathUtils  of  int32  { }
```

```ds expected
extension MathUtils of int32 {}
```
