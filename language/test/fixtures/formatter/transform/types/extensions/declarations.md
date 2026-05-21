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

## Named Extensions

### named extension

Named extensions include the name before `of`.

```ds
extension  MathUtils  of  int32  { }
```

```ds expected
extension MathUtils of int32 {}
```
