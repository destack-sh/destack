# Interface Generics

## Generics

### generic interface

Generic interfaces have type parameters in angle brackets.

```tspp
interface Container<T> { value: T }
```

```tspp expected
interface Container<T> {
    value: T;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```tspp
interface Container<T: Comparable> { value: T }
```

```tspp expected
interface Container<T: Comparable> {
    value: T;
}
```

### variance parameters

Variance modifiers precede type parameter names.

```tspp
interface   Box< in  T , out U > { get(): U; set(value: T): void }
```

```tspp expected
interface Box<in T, out U> {
    get(): U;
    set(value: T): void;
}
```

### multiple type parameters

Multiple type parameters are separated by commas.

```tspp
interface Map<K, V> { get(key: K): V; set(key: K, value: V): void }
```

```tspp expected
interface Map<K, V> {
    get(key: K): V;
    set(key: K, value: V): void;
}
```
