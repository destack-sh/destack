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

- array literal holes are not allowed

### array literal infers a union element type

> Array literals infer union element types from heterogeneous elements.

```ds
let values = [1, "two"];
values satisfies (int | string)[];
```

### array literal uses contextual element typing

> Array literal elements are checked against the contextual array element type.

```ds
let values: (int32 | string)[] = [1, "two", 3];
values satisfies (int32 | string)[];
```

### array literal rejects elements outside contextual type

> Contextual array typing rejects elements outside the target element type.

```ds
let values: int32[] = [1, "two", 3];
```

- contains: not assignable

### nested array literals preserve nested element unions

> Nested array literals infer nested union element types.

```ds
let values = [[1, 2], ["a", "b"]];
values satisfies (int32[] | string[])[];
```
