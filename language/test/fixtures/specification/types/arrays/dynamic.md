# Dynamic Arrays

## assignability

### dynamic arrays accept compatible element types

Dynamic arrays are assignable when element types are compatible.

```ds
let source: int32[] = [1, 2, 3];
let target: int32[] = source;
```

### dynamic arrays reject incompatible element types

Dynamic arrays reject assignment when element types are incompatible.

```ds
let source: int32[] = [1, 2, 3];
let target: string[] = source;
```

- contains: not assignable

### packed arrays are explicit

Packed arrays use a separate compact storage type.

```ds
let values: PackedArray<int32> = PackedArray.from([1, 2, 3]);

values satisfies PackedArray<int32>;
```

### dynamic arrays convert to packed arrays explicitly

Stable and packed arrays convert through `Into`.

```ds
let stable: Array<int32> = [1, 2, 3];
let packed: PackedArray<int32> = stable.into();

packed satisfies PackedArray<int32>;
```

### packed arrays convert to dynamic arrays explicitly

Packed arrays can be copied back into stable arrays.

```ds
let packed: PackedArray<int32> = PackedArray.from([1, 2, 3]);
let stable: Array<int32> = packed.into();

stable satisfies Array<int32>;
```

### accessors preserve receiver access

Array accessors return the form requested by the receiver.

```ds
let values: Array<int32> = [1, 2, 3];

values.first() satisfies int32 | undefined;
(&values).first() satisfies &int32 | undefined;
(&readonly values).first() satisfies &readonly int32 | undefined;
(&exclusive values).first() satisfies &exclusive int32 | undefined;
```

## indexing

### dynamic array indexing yields the element type

Indexing a dynamic array yields its element type.

```ds
let values: int32[] = [1, 2, 3];

const first = values[0];
first satisfies int32;
```

### dynamic array indexing rejects incompatible element expectations

Indexed dynamic array elements reject incompatible target types.

```ds
let values: int32[] = [1, 2, 3];

const first: string = values[0];
```

- contains: not assignable

## calls

### dynamic arrays satisfy compatible function parameters

Dynamic arrays are accepted by parameters with the same element type.

```ds
function take(values: int32[]): int32[] {
    values
}

const result = take([1, 2, 3]);
result satisfies int32[];
```

### dynamic arrays reject incompatible function parameters

Dynamic arrays reject function parameters with incompatible element types.

```ds
function take(values: string[]): string[] {
    values
}

take([1, 2, 3]);
```

- contains: not assignable
