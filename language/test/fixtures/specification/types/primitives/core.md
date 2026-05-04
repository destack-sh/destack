# Primitive Types

Primitive type assignability.

## numbers

### number to number

> Number type is assignable to itself.

```ds
const value: number = 42;
value satisfies number;
```

### literal to number

> Literal 42 is assignable to number.

```ds
const value = 42;
value satisfies number;
```

## strings

### string to string

> String type is assignable to itself.

```ds
const value: string = "hello";
value satisfies string;
```

### literal to string

> Literal "hello" is assignable to string.

```ds
const value = "hello";
value satisfies string;
```

## booleans

### boolean to boolean

> Boolean type is assignable to itself.

```ds
const value: boolean = true;
value satisfies boolean;
```

### literal to boolean

> Literal true is assignable to boolean.

```ds
const value = true;
value satisfies boolean;
```
