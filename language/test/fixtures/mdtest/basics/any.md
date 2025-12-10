# Any Type

Tests for the `any` type.

## Any Accepts Everything

### number to any

> Any type accepts number values.

```ds
const x: any = 42;
x satisfies any;
```

### string to any

> Any type accepts string values.

```ds
const x: any = "hello";
x satisfies any;
```

### object to any

> Any type accepts object values.

```ds
const x: any = { a: 1 };
x satisfies any;
```

## Any is Assignable to Everything

### any to number

> Any is assignable to number (unsafe but allowed).

```ds
const x: any = 42;
const y: number = x;
y satisfies number;
```

### any to string

> Any is assignable to string (unsafe but allowed).

```ds
const x: any = "hello";
const y: string = x;
y satisfies string;
```
