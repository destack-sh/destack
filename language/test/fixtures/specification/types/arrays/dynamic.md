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
