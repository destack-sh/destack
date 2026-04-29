# Extension Declarations

Tests for Destack extension declaration formatting.

## Basic Extensions

### simple extension

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

## Generic Extensions

### generic extension with methods

Generic extensions keep static parameters and format member bodies.

```ds
extension<T> of Box<T> { map<U>(f: (T) => U): Box<U> { return Box { value: f(this.value) } } }
```

```ds expected
extension<T> of Box<T> {
    map<U>(f: (T) => U): Box<U> {
        return Box { value: f(this.value) };
    }
}
```

### extension method tail expression

Value-returning extension methods keep terminal expressions semicolonless.

```ds
extension<T> of Box<T> { clone(): Box<T> { Box { value: this.value } } clear(): void { reset() } }
```

```ds expected
extension<T> of Box<T> {
    clone(): Box<T> {
        Box { value: this.value }
    }
    clear(): void {
        reset();
    }
}
```

### extension with implements and where clause

Extensions can include implements and where constraints.

```ds
extension<T> of Buffer<T> implements Iterable<T> where T: Copy { }
```

```ds expected
extension<T> of Buffer<T> implements Iterable<T> where T: Copy {}
```
