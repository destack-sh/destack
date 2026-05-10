# Extensions

## declarations

### extension type parameters

Generic parameters on extensions flow into member signatures.

```ds
struct Box<T> {
    value: T;
}

extension<T> of Box<T> {
    get(): T {
        return this.value;
    }
}

declare function makeBox(): Box<number>;

const boxed = makeBox();
boxed.get() satisfies number;
```

### extension static value parameters

Static value parameters on extensions are validated.

```ds
struct Buffer<T, comptime N: number> {
    value: T;
}

extension<T, comptime N: number> of Buffer<T, N> {
    get(): T {
        return this.value;
    }
}

declare function makeBuffer(): Buffer<string, 4>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension generic parameters map by target argument order

Extension parameters follow the target type argument order.

```ds
struct Pair<A, B> {
    left: A;
    right: B;
}

extension<Left, Right> of Pair<Right, Left> {
    swap(): Pair<Left, Right> {
        return Pair<Left, Right> {
            left: this.right,
            right: this.left,
        };
    }
}

declare function makePair(): Pair<number, string>;

const pair = makePair();
pair.swap() satisfies Pair<string, number>;
```

### extension static value parameters use defaults

Extensions inherit default comptime arguments from target type references.

```ds
struct Buffer<T, comptime N: number = 4> {
    value: T;
}

extension<T, comptime N: number> of Buffer<T, N> {
    get(): T {
        return this.value;
    }
}

declare function makeBuffer(): Buffer<string>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension generic parameter mismatch rejects incompatible calls

Extension methods still enforce substituted generic parameter contracts.

```ds
struct Buffer<T, comptime N: number> {
    value: T;
}

extension<T, comptime N: number> of Buffer<T, N> {
    requireSize(value: [T; N]): [T; N] {
        return value;
    }
}

declare function makeBuffer(): Buffer<uint8, 4>;

const buffer = makeBuffer();
buffer.requireSize([1, 2, 3, 4]);
buffer.requireSize([1, 2]);
```

- contains: not assignable

### extension defaults keep concrete member types

Defaulted comptime arguments are visible inside extension methods.

```ds
struct Registry<T, comptime N: number = 2> {
    value: T;
}

extension<T, comptime N: number> of Registry<T, N> {
    pair(): (T, T) {
        (this.value, this.value)
    }
}

declare function makeRegistry(): Registry<string>;

const registry = makeRegistry();
registry.pair() satisfies (string, string);
```
