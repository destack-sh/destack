# Array Literals

Array literal type inference and checking.

## arrays

### array of numbers

> Arrays of numbers infer element type as number.

```ds
const x = [1, 2, 3];
x satisfies number[];
```

### array of strings

> Arrays of strings infer element type as string.

```ds
const x = ["a", "b", "c"];
x satisfies string[];
```

### empty array

> Empty arrays have unknown element type.

```ds
const x = [];
x satisfies unknown[];
```

### mixed array

> Arrays with mixed types infer a union element type.

```ds
const x = [1, "two", true];
x satisfies (number | string | boolean)[];
```

### array holes are rejected

> Array literals are dense and do not allow holes.

```ds
const x = [1, , 3];
```

- contains: array literal holes

## contextual arrays

### contextual array literal

> Array literals use contextual types for element inference.

```ds
const values: number[] = [1, 2, 3];
values satisfies number[];
```

### contextual array literal mismatch

> Array literal elements must satisfy contextual element types.

```ds
const values: number[] = [1, "two"];
```

- contains: not assignable

### contextual array literal via alias

> Contextual array element types are preserved through aliases.

```ds
type Numbers = number[];

const values: Numbers = [1, 2, 3];
values satisfies number[];
```

### contextual array literal alias mismatch

> Alias contextual element types still enforce element constraints.

```ds
type Numbers = number[];

const values: Numbers = [1, "two"];
```

- contains: not assignable

### contextual array spread literal

> Spread arrays use contextual element types for the resulting array.

```ds
const values: number[] = [...[1, 2]];
values satisfies number[];
```

### contextual array spread literal mismatch

> Spread arrays still enforce contextual element constraints.

```ds
const values: number[] = [...[1, "two"]];
```

- contains: not assignable

## named array types

### Array<T> matches array syntax


```ds
const values: Array<number> = [1, 2, 3];
values satisfies number[];
```

### array syntax matches Array<T>


```ds
const values: number[] = [1, 2, 3];
values satisfies Array<number>;
```

## array spreads

### array spread preserves element types

> Array spreads keep element types for the merged literal.

```ds
const base = [1, 2];
const values = [...base, 3];
values satisfies number[];
```

## index access

### index access returns element type

> Array index access is bounds checked and returns the element type.

```ds
const values = [1, 2, 3];
let value: number = values[0];
```

## array members

### array length resolves

> Arrays expose length.

```ds
const values = [1, 2, 3];
values.length satisfies int32;
```

### array push checks element types

> Array mutation methods enforce element types.

```ds
const values: number[] = [1, 2, 3];
values.push("no");
```

- contains: not assignable
