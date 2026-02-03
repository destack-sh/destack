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

- contains: not assignable

### sized arrays require compatible element types

> Sized arrays require element type compatibility when converting to dynamic arrays.

```ds
let fixed: int32[2] = [1, 2];
let dynamic: string[] = fixed;
```

- contains: not assignable

### sized arrays are usable in function arguments

> Sized arrays are assignable to dynamic arrays in function calls.

```ds
function take(values: int32[]): int32[] { return values; }
let fixed: int32[2] = [1, 2];
let dynamic = take(fixed);
```
