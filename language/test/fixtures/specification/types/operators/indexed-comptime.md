# Indexed Access And Comptime

## disambiguation

### numeric keys on object types stay indexed access in ds

In `.ds`, indexing object types with numeric keys should remain object indexed access, not tuple reinterpretation.

```ds
type Pair = { 0: string, 1: int32 };
type Right = Pair[1];

declare const value: Right;
value satisfies int32;
```

### missing numeric object keys do not reinterpret as fixed arrays

If a numeric key is absent on an object type, the checker should report a missing property instead of reinterpreting as an array.

```ds
type ObjectLike = { label: string };
type Missing = ObjectLike[5];
```

- does not exist

### as comptime forces fixed array construction for static values

`as comptime` should force static tuple-like construction so numeric indexing is resolved against fixed positions.

```ds
type Buffer<comptime N: number> = uint8[N as comptime];

declare const value: Buffer<4>;
value satisfies uint8[4];
```

### ambiguous index space requires explicit as comptime in ds

When both object-style and fixed-array indexing are plausible, `.ds` should require explicit `as comptime` disambiguation.

```ds
type Resolve<T, N> = T[N];
```

- ambiguous

### imported associated comptime lengths need as comptime when indexed access is admissible

Associated comptime lengths that cross module boundaries should still require `as comptime` when ordinary indexed access is also legal.

```ds:layout.ds
export interface WindowLayout<T> {
    comptime const Count: number = 4;
    type Window = T[this.Count as comptime];
}

export class ByteWindowLayout implements WindowLayout<uint8[]> {}
```

```ds:main.ds
import { ByteWindowLayout } from "./layout";

declare const window: ByteWindowLayout.Window;
window satisfies uint8[][4 as comptime];
```
