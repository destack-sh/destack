# Array Literals

Tests for array literal type inference and checking.

## Basic Arrays

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
x satisfies [];
```

### mixed array

> Arrays with mixed types infer a union element type.

```ds
const x = [1, "two", true];
x satisfies (number | string | boolean)[];
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

- contains: not assignable to type number[]

## Named Array Types

### Array<T> matches array syntax


```ds libs=es5
const values: Array<number> = [1, 2, 3];
values satisfies number[];
```

### array syntax matches Array<T>


```ds libs=es5
const values: number[] = [1, 2, 3];
values satisfies Array<number>;
```

## index access

### noUncheckedIndexedAccess adds undefined to array reads

```ds
const values = [1, 2, 3];
let value: number = values[0];
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
const values = [1, 2, 3];
let value: number = values[0];
```

## Array Members

### array filter resolves

> Arrays expose filter with typed results.


```ds libs=es5
const values = [1, 2, 3];
const filtered = values.filter(value => value > 1);
filtered satisfies number[];
```

### array findIndex resolves

> Arrays expose findIndex.


```ds libs=es2015
const values = [1, 2, 3];
const index = values.findIndex(value => value > 1);
index satisfies number;
```
