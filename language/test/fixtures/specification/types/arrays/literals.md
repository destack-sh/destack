# Array Literals

## dynamic arrays

### let array literals infer widened element arrays

Let array literals infer widened dynamic element types.

```ds
let values = [1, 2, 3];
values satisfies number[];
```

### const array literals infer widened element arrays without const assertions

Const array literals still widen to dynamic arrays without explicit const assertions.

```ds
const values = [1, 2, 3];
values satisfies number[];
```

## fixed arrays

### fixed array annotations accept matching literal lengths

Fixed array annotations accept literals with matching lengths.

```ds
const pair: [int32; 2] = [1, 2];
pair satisfies [int32; 2];
```

### fixed array length holes infer literal length

Fixed array length holes infer their length from an array literal context.

```ds
const bytes: [uint8; _] = [1, 2, 3, 4];
bytes satisfies [uint8; 4];
```

### fixed array element holes infer widened element type

Fixed array element holes infer the best common widened element type.

```ds
const values: [_; 3] = [1, 2, 3];
values satisfies [int32; 3];
```

### fixed array full holes infer widened element type and length

Fixed array element and length holes infer a homogeneous fixed array type.

```ds
const values: [_; _] = [1, 2, 3, 4];
values satisfies [int32; 4];
```

### explicit literal union elements preserve literal union context

Literal unions are preserved when the fixed array element context asks for them.

```ds
const values: [1 | 2 | 3; 3] = [1, 2, 3];
values satisfies [1 | 2 | 3; 3];
```

### fixed array casts infer element type and length

Fixed array casts provide the same context as fixed array annotations.

```ds
const values = [1, 2, 3] as [_; _];
values satisfies [int32; 3];
```

### slice casts infer widened element type

Slice casts infer the widened element type but not a fixed length.

```ds
const values = [1, 2, 3] as Slice<_>;
values satisfies Slice<int32>;
```

### bracket slice casts infer widened element type

Bracket slice casts infer the widened element type just like `Slice<_>`.

```ds
const values = [1, 2, 3] as [_];
values satisfies Slice<int32>;
```

### fixed array annotations reject mismatched literal lengths

Fixed array annotations reject literals with mismatched lengths.

```ds
const pair: [int32; 2] = [1, 2, 3];
```

- contains: not assignable

### nested fixed array annotations accept matching nested lengths

Nested fixed array annotations accept matching nested literal shapes.

```ds
const matrix: [[int32; 2]; 2] = [
    [1, 2],
    [3, 4],
];
matrix satisfies [[int32; 2]; 2];
```

### nested fixed array annotations reject mismatched nested lengths

Nested fixed array annotations reject mismatched nested literal shapes.

```ds
const matrix: [[int32; 2]; 2] = [[1, 2], [3]];
```

- contains: not assignable
