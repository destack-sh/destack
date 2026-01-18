# Static Value Parameters

Tests for static value parameters used in type expressions.

## value parameters

### static value parameters apply to sized arrays

> Value parameters can drive array sizes in type expressions.

```ds
type Buffer<N: number> = uint8[N];

declare let value: Buffer<4>;

value satisfies uint8[4];
```

### static value parameters flow through type aliases

> Static value parameters can be passed through type aliases.

```ds
type Buffer<N: number> = uint8[N];
type Outer<M: number> = Buffer<M>;

declare let value: Outer<4>;

value satisfies uint8[4];
```

### static value defaults can reference earlier parameters

> Default value parameters may reference earlier parameters.

```ds
type Buffer<N: number, M: number = N> = uint8[M];

declare let value: Buffer<4>;

value satisfies uint8[4];
```
