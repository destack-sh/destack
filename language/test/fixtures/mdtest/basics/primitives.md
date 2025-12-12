# Primitive Types

Tests for primitive type assignability.

## Number

### number to number

> Number type is assignable to itself.

```ds
const value: number = 42;
value satisfies number;
```

### literal to number

> Literal 42 should be assignable to number.

```ds
const value = 42;
value satisfies number;
```

## String

### string to string

> String type is assignable to itself.

```ds
const value: string = "hello";
value satisfies string;
```

### literal to string

> Literal "hello" should be assignable to string.

```ds
const value = "hello";
value satisfies string;
```

## Boolean

### boolean to boolean

> Boolean type is assignable to itself.

```ds
const value: boolean = true;
value satisfies boolean;
```

### literal to boolean

> Literal true should be assignable to boolean.

```ds
const value = true;
value satisfies boolean;
```
