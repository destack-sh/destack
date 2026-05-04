# Array Literal Commitment

## dynamic array commitment

### let array literals commit to widened element arrays

> Let array literals commit to widened dynamic element types.

```ds
let values = [1, 2, 3];
values satisfies int32[];
```

### const array literals still commit to widened element arrays without const assertions

> Const array literals still widen to dynamic arrays without explicit const assertions.

```ds
const values = [1, 2, 3];
values satisfies int32[];
```

## fixed array commitment

### fixed array annotations accept matching literal lengths

> Fixed-size array annotations accept literals with matching lengths.

```ds
const pair: [int32; 2] = [1, 2];
pair satisfies [int32; 2];
```

### fixed array annotations reject mismatched literal lengths

> Fixed-size array annotations reject literals with mismatched lengths.

```ds
const pair: [int32; 2] = [1, 2, 3];
```

- contains: not assignable

### nested fixed array annotations accept matching nested lengths

> Nested fixed-size array annotations accept matching nested literal shapes.

```ds
const matrix: [[int32; 2]; 2] = [[1, 2], [3, 4]];
matrix satisfies [[int32; 2]; 2];
```

### nested fixed array annotations reject mismatched nested lengths

> Nested fixed-size array annotations reject mismatched nested literal shapes.

```ds
const matrix: [[int32; 2]; 2] = [[1, 2], [3]];
```

- contains: not assignable
