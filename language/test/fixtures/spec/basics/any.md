# Any Type

Tests for the `any` type.

## Any Accepts Everything

### number to any

> Any type accepts number values.

```ds
const value: any = 42;
value satisfies any;
```

### string to any

> Any type accepts string values.

```ds
const value: any = "hello";
value satisfies any;
```

### object to any

> Any type accepts object values.

```ds
const value: any = { a: 1 };
value satisfies any;
```

## Any is Assignable to Everything

### any to number

> Any is assignable to number (unsafe but allowed).

```ds
const anyValue: any = 42;
const numberValue: number = anyValue;
numberValue satisfies number;
```

### any to string

> Any is assignable to string (unsafe but allowed).

```ds
const anyValue: any = "hello";
const stringValue: string = anyValue;
stringValue satisfies string;
```
