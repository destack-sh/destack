# Class Generics

Tests for static parameters on class type references.

## classes

### explicit static type arguments on classes

> Class type references accept explicit static type arguments.

```ds
class Box<T> { value: T }

declare function make_box(): Box<number>;

let value: Box<number> = make_box();
value satisfies Box<number>;
```

### static type argument mismatch on classes

> Static type arguments must satisfy declared bounds.

```ds
class Box<T: number> { value: T }

declare function make_box(): Box<number>;

let value: Box<string> = make_box();
```

- contains: type string is not assignable to type number

### static value arguments on classes

> Static value arguments are checked against declared types.

```ds
class Buffer<T, N: number> { value: T }

declare function make_buffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = make_buffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch on classes

> Static value arguments must satisfy declared types.

```ds
class Buffer<T, N: number> { value: T }

declare function make_buffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = make_buffer();
```

- contains: type true is not assignable to type number

### default static type parameters on classes

> Static type parameters fall back to defaults when omitted.

```ds
class Box<T = number> { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on classes

> Static value arguments fall back to defaults when omitted.

```ds
class Buffer<T, N: number = 4> { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
