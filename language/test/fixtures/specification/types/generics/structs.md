# Struct Generics

Tests for static parameters on struct type references.

## structs

### explicit static type arguments on structs

> Struct type references accept explicit static type arguments.

```ds
struct Box<T> { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on structs

> Static type arguments must satisfy declared bounds.

```ds
struct Box<T: number> { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on structs

> Static value arguments are checked against declared types.

```ds
struct Buffer<T, N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch on structs

> Static value arguments must satisfy declared types.

```ds
struct Buffer<T, N: number> { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on structs

> Static type parameters fall back to defaults when omitted.

```ds
struct Box<T = number> { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on structs

> Static value arguments fall back to defaults when omitted.

```ds
struct Buffer<T, N: number = 4> { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
