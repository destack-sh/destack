# Primitive Types

Tests for primitive type assignability.

## Number

### number to number

> Number type is assignable to itself.

```ds
const x: number = 42;
let y: number = x;
y satisfies number;
```

### literal to number

> Literal 42 should be assignable to number.

```ds
const x = 42;
let y: number = x;
y satisfies number;
```

## String

### string to string

> String type is assignable to itself.

```ds
const x: string = "hello";
let y: string = x;
y satisfies string;
```

### literal to string

> Literal "hello" should be assignable to string.

```ds
const x = "hello";
let y: string = x;
y satisfies string;
```

## Boolean

### boolean to boolean

> Boolean type is assignable to itself.

```ds
const x: boolean = true;
let y: boolean = x;
y satisfies boolean;
```

### literal to boolean

> Literal true should be assignable to boolean.

```ds
const x = true;
let y: boolean = x;
y satisfies boolean;
```
