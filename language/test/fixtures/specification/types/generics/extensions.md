# Extension Generics

Tests for static parameters on extensions.

## extensions

### extension static type parameters

> Static parameters on extensions should flow into member signatures.

```ds
struct Box<T> { value: T }

extension<T> for Box<T> {
    get(): T { return this.value }
}

declare function makeBox(): Box<number>;

const boxed = makeBox();
boxed.get() satisfies number;
```

### extension static value parameters

> Static value parameters on extensions should be validated.

```ds
struct Buffer<T, N: number> { value: T }

extension<T, N> for Buffer<T, N> {
    get(): T { return this.value }
}

declare function makeBuffer(): Buffer<string, 4>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension static parameters map by target argument order

> Extension parameters follow the target type argument order.

```ds
struct Pair<A, B> { left: A, right: B }

extension<Left, Right> for Pair<Right, Left> {
    swap(): Pair<Left, Right> { return Pair<Left, Right> { left: this.right, right: this.left } }
}

declare function makePair(): Pair<number, string>;

const pair = makePair();
pair.swap() satisfies Pair<string, number>;
```

### extension static value parameters use defaults

> Extensions inherit default static arguments from target type references.

```ds
struct Buffer<T, N: number = 4> { value: T }

extension<T, N> for Buffer<T, N> {
    get(): T { return this.value }
}

declare function makeBuffer(): Buffer<string>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```
