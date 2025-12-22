# Type Alias Generics

Tests for static parameters on type aliases and newtypes.

## type aliases

### explicit static type arguments on type aliases

> Type aliases accept explicit static type arguments.

```ds
type Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on type aliases

> Static type arguments must satisfy declared bounds.

```ds
type Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on type aliases

> Static value arguments are checked against declared types.

```ds
type Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch on type aliases

> Static value arguments must satisfy declared types.

```ds
type Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on type aliases

> Static type parameters fall back to defaults when omitted.

```ds
type Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on type aliases

> Static value arguments fall back to defaults when omitted.

```ds
type Buffer<T, N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

## newtypes

### explicit static type arguments on newtypes

> Newtypes accept explicit static type arguments.

```ds
newtype Box<T> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### static type argument mismatch on newtypes

> Static type arguments must satisfy declared bounds.

```ds
newtype Box<T: number> = { value: T }

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: type string is not assignable to type number

### static value arguments on newtypes

> Static value arguments are checked against declared types.

```ds
newtype Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### static value argument mismatch on newtypes

> Static value arguments must satisfy declared types.

```ds
newtype Buffer<T, N: number> = { value: T }

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: type true is not assignable to type number

### default static type parameters on newtypes

> Static type parameters fall back to defaults when omitted.

```ds
newtype Box<T = number> = { value: T }

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### default static value arguments on newtypes

> Static value arguments fall back to defaults when omitted.

```ds
newtype Buffer<T, N: number = 4> = { value: T }

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
