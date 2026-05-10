# Generic Extensions

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
