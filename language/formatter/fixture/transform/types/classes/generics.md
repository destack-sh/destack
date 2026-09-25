# Class Generics

## Generics

### generic class

Generic classes have type parameters in angle brackets.

```tspp
class Container<T> { value: T }
```

```tspp expected
class Container<T> {
    value: T;
}
```

### generic class with constraint

Type constraints use colon syntax: `T: Constraint`.

```tspp
class Container<T: Comparable> { value: T }
```

```tspp expected
class Container<T: Comparable> {
    value: T;
}
```

### generic class with multiple type params

Multiple type parameters are separated by commas.

```tspp
class Pair<K, V> { key: K; value: V }
```

```tspp expected
class Pair<K, V> {
    key: K;
    value: V;
}
```
