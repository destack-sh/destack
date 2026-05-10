# Generic Newtypes

Newtypes can bind type parameters and static value parameters.

## arguments

### newtypes accept explicit type arguments

Newtypes accept explicit type arguments.

```ds
newtype Box<T> = { value: T };

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
value satisfies Box<number>;
```

### newtypes reject type argument mismatches

Type arguments must satisfy declared bounds.

```ds
newtype Box<T: number> = { value: T };

declare function makeBox(): Box<number>;

let value: Box<string> = makeBox();
```

- contains: not assignable

### newtypes accept static value arguments

Static value arguments are checked against declared types.

```ds
newtype Buffer<T, comptime N: number> = { value: T };

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, 4> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```

### newtypes require static value arguments

Newtype static value arguments must be static expressions.

```ds
newtype Buffer<T, comptime N: number> = { value: T };

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, comptime 4> = makeBuffer();
```

- contains: static expression

### newtypes reject static value argument mismatches

Static value arguments must satisfy declared types.

```ds
newtype Buffer<T, comptime N: number> = { value: T };

declare function makeBuffer(): Buffer<string, 4>;

let buffer: Buffer<string, true> = makeBuffer();
```

- contains: not assignable

## defaults

### newtypes accept default type parameters

Type parameters fall back to defaults when omitted.

```ds
newtype Box<T = number> = { value: T };

declare function makeBox(): Box;

let value: Box = makeBox();
value satisfies Box<number>;
```

### newtypes accept default static values

Static value arguments fall back to defaults when omitted.

```ds
newtype Buffer<T, comptime N: number = 4> = { value: T };

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
buffer satisfies Buffer<string, 4>;
```
