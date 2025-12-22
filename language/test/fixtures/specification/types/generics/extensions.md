# Extension Generics

> #Incomplete: extension instancing does not resolve static arguments yet

Tests for static parameters on extensions.

## extensions

### _extension static type parameters

> Static parameters on extensions should flow into member signatures.

```ds
struct Box<T> { value: T }

extension for Box<T> {
    get(): T { return this.value }
}
```

### _extension static value parameters

> Static value parameters on extensions should be validated.

```ds
struct Buffer<T, N: number> { value: T }

extension for Buffer<T, N> {
    get(): T { return this.value }
}
```
