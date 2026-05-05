# Generic Structs

Structs can bind type parameters and static value parameters.

## arguments

### structs accept explicit type arguments

> Struct type references accept explicit type arguments.

```ds
struct Box<T> { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### structs reject type argument mismatches

> Type arguments must satisfy declared bounds.

```ds
struct Box<T: number> { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: not assignable

### structs accept static value arguments

> Static value arguments are checked against declared types.

```ds
struct Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### structs reject static value argument mismatches

> Static value arguments must satisfy declared types.

```ds
struct Buffer<T, comptime N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: not assignable

## defaults

### structs accept default type parameters

> Type parameters fall back to defaults when omitted.

```ds
struct Box<T = number> { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### structs accept default static values

> Static value arguments fall back to defaults when omitted.

```ds
struct Buffer<T, comptime N: number = 4> { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
