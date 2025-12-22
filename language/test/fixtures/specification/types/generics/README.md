# Type Generics

Tests for static parameter instancing on type references.

## type references

### explicit static type arguments on type references

> Type references accept explicit static arguments.

```ds
struct Box<T> { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static value arguments on type references

> Static value arguments are checked against declared types.

```ds
struct Buffer<T, N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch

> Static value arguments must satisfy declared types.

```ds
struct Buffer<T, N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### inherited static arguments on member calls

> Member calls apply inherited static arguments from the receiver type.

```ds
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

getContainer().map<string>("hello");
```

- contains: type "hello" is not assignable to type number
