# Array Expressions

## array literals

### array literal assigns to array type

> Array literals can be assigned to array types.

```ds
let values: int32[] = [1, 2, 3];
```

### array literal rejects holes

> Array literals do not permit holes.

```ds
let bad = [1,, 3];
```

- contains: array literal holes are not allowed