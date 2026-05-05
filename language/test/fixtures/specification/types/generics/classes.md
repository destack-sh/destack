# Generic Classes

Classes can bind type parameters and static value parameters.

## arguments

### classes accept explicit type arguments

> Class type references accept explicit type arguments.

```ds
class Box<T> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### classes reject type argument mismatches

> Type arguments must satisfy declared bounds.

```ds
class Box<T: number> {
    value?: T
}

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: not assignable

### classes accept static value arguments

> Static value arguments are checked against declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### classes reject static value argument mismatches

> Static value arguments must satisfy declared types.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: not assignable

## defaults

### classes accept default type parameters

> Type parameters fall back to defaults when omitted.

```ds
class Box<T = number> {
    value?: T
}

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### classes accept default static values

> Static value arguments fall back to defaults when omitted.

```ds
class Buffer<T, comptime N: number = 4> {
    value?: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

## assignability

### type arguments affect class assignability

> Class type references with different type arguments are not assignable.

```ds
class Box<T> {
    value?: T
}

declare let numberBox: Box<number>;

let stringBox: Box<string> = numberBox;
```

- contains: not assignable

### static value arguments affect class assignability

> Class references with different value arguments are not assignable.

```ds
class Buffer<T, comptime N: number> {
    value?: T
}

declare let buffer4: Buffer<string, 4>;

let buffer8: Buffer<string, 8> = buffer4;
```

- contains: not assignable
