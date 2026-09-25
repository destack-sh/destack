# Interface Signatures

## Call and Construct Signatures

### call signature

Call signatures make an interface callable like a function.

```tspp
interface Callable { (x: number): number }
```

```tspp expected
interface Callable {
    (x: number): number;
}
```

### construct signature

Construct signatures allow using `new` with the interface.

```tspp
interface Constructor { new(x: number): Foo }
```

```tspp expected
interface Constructor {
    new (x: number): Foo;
}
```


## Signatures

### new signature in interface

Constructor signatures keep a space before parameter lists.

```tspp:main.tspp
interface Creator { new(...args): Foo }
```

```tspp expected
interface Creator {
    new (...args): Foo;
}
```

### call signature in interface

Call signatures format without a name and include semicolons.

```tspp:main.tspp
interface Callable { (...args): Foo }
```

```tspp expected
interface Callable {
    (...args): Foo;
}
```

### callable boolean signature in declaration interface

Declaration interfaces can expose callable boolean signatures.

```tspp
interface Guard<T> { (value: unknown): boolean; readonly source?: string }
```

```tspp expected
interface Guard<T> {
    (value: unknown): boolean;
    readonly source?: string;
}
```
