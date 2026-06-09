# Fixed Arrays

## fixed to dynamic arrays

### fixed arrays are assignable to dynamic arrays

Fixed arrays are assignable to dynamic arrays.

```ds
let fixed: [int32; 3] = [1, 2, 3];
let dynamic: int32[] = fixed;
```

### dynamic arrays are not assignable to fixed arrays

Dynamic arrays are not assignable to fixed arrays.

```ds
let dynamic: int32[] = [1, 2, 3];
let fixed: [int32; 3] = dynamic;
```

- contains: not assignable

### fixed arrays require compatible element types

Fixed arrays require element type rules when converting to dynamic arrays.

```ds
let fixed: [int32; 2] = [1, 2];
let dynamic: string[] = fixed;
```

- contains: not assignable

### fixed arrays are usable in function arguments

Fixed arrays are assignable to dynamic arrays in function calls.

```ds
function take(values: int32[]): int32[] {
    return values;
}
let fixed: [int32; 2] = [1, 2];
let dynamic = take(fixed);
```

### fixed arrays with equal lengths are assignable

Fixed arrays with equal lengths and element types are assignable.

```ds
let source: [int32; 2] = [1, 2];
let target: [int32; 2] = source;
```

### fixed arrays with different lengths are not assignable

Fixed arrays with different lengths are not assignable.

```ds
let source: [int32; 2] = [1, 2];
let target: [int32; 3] = source;
```

- contains: not assignable

## builtin fixed array alias

### FixedArray is an alias for fixed arrays

`FixedArray<T, N>` is a builtin alias for `[T; N]`.

```ds
const lane: FixedArray<int32, 2> = [1, 2];
lane satisfies [int32; 2];
```

### FixedArray keeps the declared length

`FixedArray<T, N>` rejects literals whose length does not match `N`.

```ds
const lane: FixedArray<int32, 2> = [1, 2, 3];
```

- contains: not assignable

### FixedArray exposes fixed array members

`FixedArray<T, N>` exposes the same fixed array member surface as `[T; N]`.

```ds
const lane: FixedArray<int32, 2> = [1, 2];
lane.size satisfies usize;
```

### fixed arrays expose FixedArray members

`[T; N]` resolves members through the builtin `FixedArray<T, N>` alias.

```ds
const lane: [int32; 2] = [1, 2];
lane.size satisfies usize;
```
