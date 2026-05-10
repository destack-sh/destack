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

## Named Extensions

### named extension

Named extensions include the name before `of`.

```ds
extension  MathUtils  of  int32  { }
```

```ds expected
extension MathUtils of int32 {}
```
