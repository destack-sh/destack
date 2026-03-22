# Sized Arrays

## fixed-size to dynamic arrays

### sized arrays are assignable to dynamic arrays

> Fixed-size arrays are assignable to dynamic arrays.

```ds
let fixed: int32[3] = [1, 2, 3];
let dynamic: int32[] = fixed;
```

### dynamic arrays are not assignable to sized arrays

> Dynamic arrays are not assignable to fixed-size arrays.

```ds
let dynamic: int32[] = [1, 2, 3];
let fixed: int32[3] = dynamic;
```

- not assignable

### sized arrays require compatible element types

> Sized arrays require element type compatibility when converting to dynamic arrays.

```ds
let fixed: int32[2] = [1, 2];
let dynamic: string[] = fixed;
```

- not assignable

### sized arrays are usable in function arguments

> Sized arrays are assignable to dynamic arrays in function calls.

```ds
function take(values: int32[]): int32[] { return values; }
let fixed: int32[2] = [1, 2];
let dynamic = take(fixed);
```

### sized arrays with equal lengths are assignable

> Fixed arrays with equal lengths and element types are assignable.

```ds
let source: int32[2] = [1, 2];
let target: int32[2] = source;
```

### sized arrays with different lengths are not assignable

> Fixed arrays with different lengths are not assignable.

```ds
let source: int32[2] = [1, 2];
let target: int32[3] = source;
```

- not assignable

## builtin fixed array alias

### FixedArray alias commits to fixed-size arrays

> `FixedArray<T, N>` is a builtin alias for explicit fixed-size array intent.

```ds libs=native
const lane: FixedArray<int32, 2> = [1, 2];
lane satisfies int32[2];
```

### FixedArray alias enforces declared length

> `FixedArray<T, N>` rejects literals whose length does not match `N`.

```ds libs=native
const lane: FixedArray<int32, 2> = [1, 2, 3];
```

- not assignable
