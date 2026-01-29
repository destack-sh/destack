# Static Arguments for Classes

Tests for static parameters on class type references.

## classes

### explicit static type arguments on classes

> Class type references accept explicit static type arguments.

```ds
class Box<T> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on classes

> Static type arguments must satisfy declared bounds.

```ds
class Box<T: number> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on classes

> Static value arguments are checked against declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch on classes

> Static value arguments must satisfy declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on classes

> Static type parameters fall back to defaults when omitted.

```ds
class Box<T = number> {
    value?: T
}

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on classes

> Static value arguments fall back to defaults when omitted.

```ds
class Buffer<T, comptime N: number = 4> {
    value?: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static type arguments affect assignability

> Class type references with different static type arguments are not assignable.

```ds
class Box<T> {
    value?: T
}

declare let numberBox: Box<number>;

let stringBox: Box<string> = numberBox;
```

- contains: not assignable

### static value arguments affect assignability

> Class references with different value arguments are not assignable.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare let buffer4: Buffer<string, 4>;

let buffer8: Buffer<string, 8> = buffer4;
```

- contains: not assignable
