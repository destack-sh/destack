# Array Literals

Tests for array literal type inference and checking.

## Basic Arrays

### array of numbers

> Arrays of numbers infer element type as number.

```ds
const x = [1, 2, 3];
x satisfies [1, 2, 3];
```

### array of strings

> Arrays of strings infer element type as string.

```ds
const x = ["a", "b", "c"];
x satisfies ["a", "b", "c"];
```

### empty array

> Empty arrays have unknown element type.

```ds
const x = [];
x satisfies [];
```

### mixed array

> Arrays with mixed types infer a union element type.

```ds
const x = [1, "two", true];
x satisfies [1, "two", true];
```

## Contextual Arrays

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

- contains: type (number, "two") is not assignable to type number[]

## index access

### noUncheckedIndexedAccess adds undefined to array reads

```ds
const values = [1, 2, 3]
let value: number = values[0]
```

- contains: not assignable

### noUncheckedIndexedAccess false allows array reads

```ds:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const values = [1, 2, 3]
let value: number = values[0]
```
