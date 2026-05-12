# Slices

Slices are runtime-length homogeneous views.

## syntax

### bracket slice syntax names slice types

`[T]` is a slice of `T`.

```ds
declare const values: [int32];
values satisfies Slice<int32>;
```

### slice alias matches bracket syntax

`Slice<T>` is the library alias for `[T]`.

```ds
declare const values: Slice<int32>;
values satisfies [int32];
```

### borrowed slices keep the slice view shape

Borrowing a slice preserves the pointer-length view.

```ds
declare const values: [int32];

const borrow: &[int32] = &values;
borrow satisfies &[int32];
```

## indexing

### slice indexing yields element types

Indexing a slice yields its element type.

```ds
declare const values: [int32];

const first = values[0];
first satisfies int32;
```

### slices are not fixed arrays

Runtime-length slices do not satisfy compile-time fixed arrays.

```ds
declare const values: [int32];
const fixed: [int32; 2] = values;
```

- contains: not assignable
