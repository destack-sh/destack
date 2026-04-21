# Static Arguments for Extensions

Tests for static parameters on extensions.

## extensions

### extension static type parameters

> Static parameters on extensions should flow into member signatures.

```ds
struct Box<T> { value: T }

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

> Static value parameters on extensions should be validated.

```ds
struct Buffer<T, comptime N: number> { value: T }

extension<T, comptime N: number> of Buffer<T, N> {
    get(): T {
        return this.value;
    }
}

declare function makeBuffer(): Buffer<string, 4>;

const buffer = makeBuffer();
buffer.get() satisfies string;
```

### extension static parameters map by target argument order

> Extension parameters follow the target type argument order.

```ds
struct Pair<A, B> {
    left: A;
    right: B
}

extension<Left, Right> of Pair<Right, Left> {
    swap(): Pair<Left, Right> {
        return Pair<Left, Right> {
            left: this.right,
            right: this.left
        };
    }
}

declare function makePair(): Pair<number, string>;

const pair = makePair();
pair.swap() satisfies Pair<string, number>;
```

### extension static value parameters use defaults

> Extensions inherit default static arguments from target type references.

```ds
struct Buffer<T, comptime N: number = 4> {
    value: T
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

### extension static parameter mismatch rejects incompatible calls

> Extension methods still enforce substituted static parameter contracts.

```ds
struct Buffer<T, comptime N: number> {
    value: T
}

extension<T, comptime N: number> of Buffer<T, N> {
    requireSize(value: T[N as comptime]): T[N as comptime] {
        return value;
    }
}

declare function makeBuffer(): Buffer<uint8, 4>;

const buffer = makeBuffer();
buffer.requireSize([1, 2, 3, 4]);
buffer.requireSize([1, 2]);
```

- contains: not assignable

### extension static defaults preserve mapped owner substitutions

> Defaulted static arguments remain specialized through extension member projections.

```ds
struct Registry<T, comptime N: number = 2> {
    value: T
}

extension<T, comptime N: number> of Registry<T, N> {
    pair(): [T, T] {
        [this.value, this.value]
    }
}

declare function makeRegistry(): Registry<string>;

const registry = makeRegistry();
registry.pair() satisfies [string, string];
```
