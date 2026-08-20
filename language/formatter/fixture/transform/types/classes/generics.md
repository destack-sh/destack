# Class Generics

## Generics

### generic class

Generic classes have type parameters in angle brackets.

```ds
class Container<T> { value: T }
```

```ds expected
class Container<T> {
    value: T;
}
```

### generic class with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
class Container<T: Comparable> { value: T }
```

```ds expected
class Container<T: Comparable> {
    value: T;
}
```

### generic class with multiple type params

Multiple type parameters are separated by commas.

```ds
class Pair<K, V> { key: K; value: V }
```

```ds expected
class Pair<K, V> {
    key: K;
    value: V;
}
```
