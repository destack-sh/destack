# Interface Signatures

## Call and Construct Signatures

### call signature

Call signatures make an interface callable like a function.

```ds
interface Callable { (x: number): number }
```

```ds expected
interface Callable {
    (x: number): number;
}
```

### construct signature

Construct signatures allow using `new` with the interface.

```ds
interface Constructor { new(x: number): Foo }
```

```ds expected
interface Constructor {
    new (x: number): Foo;
}
```


## TypeScript Signatures

### new signature in interface

TypeScript `new` signatures keep a space before parameter lists.

```ts:main.ts
interface Creator { new(...args): Foo }
```

```ts expected
interface Creator {
    new (...args): Foo;
}
```

### call signature in interface

Call signatures format without a name and include semicolons.

```ts:main.ts
interface Callable { (...args): Foo }
```

```ts expected
interface Callable {
    (...args): Foo;
}
```

### callable boolean signature in declaration interface

Declaration interfaces can expose callable boolean signatures.

```ds
interface Guard<T> { (value: unknown): boolean; readonly source?: string }
```

```ds expected
interface Guard<T> {
    (value: unknown): boolean;
    readonly source?: string;
}
```
