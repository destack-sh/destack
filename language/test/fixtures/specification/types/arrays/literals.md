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
