# Generic Extensions

## Generic Extensions

### generic extension with methods

Generic extensions keep parameters and format member bodies.

```tspp
extension<T> of Box<T> { map<U>(f: (T) => U): Box<U> { return Box { value: f(this.value) } } }
```

```tspp expected
extension<T> of Box<T> {
    map<U>(f: (T) => U): Box<U> {
        return Box { value: f(this.value) };
    }
}
```

### extension method tail expression

Value-returning extension methods keep terminal expressions semicolonless.

```tspp
extension<T> of Box<T> { clone(): Box<T> { Box { value: this.value } } clear(): void { reset() } }
```

```tspp expected
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

```tspp
extension<T> of Buffer<T> implements Iterable<T> where T: Copy { }
```

```tspp expected
extension<T> of Buffer<T> implements Iterable<T> where T: Copy {}
```
