# Interface Generics

## Generics

### generic interface

Generic interfaces have type parameters in angle brackets.

```ds
interface Container<T> { value: T }
```

```ds expected
interface Container<T> {
    value: T;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
interface Container<T: Comparable> { value: T }
```

```ds expected
interface Container<T: Comparable> {
    value: T;
}
```

### variance parameters

Variance modifiers precede type parameter names.

```ds
interface   Box< in  T , out U > { get(): U; set(value: T): void }
```

```ds expected
interface Box<in T, out U> {
    get(): U;
    set(value: T): void;
}
```

### multiple type parameters

Multiple type parameters are separated by commas.

```ds
interface Map<K, V> { get(key: K): V; set(key: K, value: V): void }
```

```ds expected
interface Map<K, V> {
    get(key: K): V;
    set(key: K, value: V): void;
}
```
