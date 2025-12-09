# Any Type

Tests for the `any` type.

## Any Accepts Everything

### number to any

> Any type accepts number values.

```ds
const x: any = 42
```

### string to any

> Any type accepts string values.

```ds
const x: any = "hello"
```

### object to any

> Any type accepts object values.

```ds
const x: any = { a: 1 }
```

## Any is Assignable to Everything

### any to number

> Any is assignable to number (unsafe but allowed).

```ds
const x: any = 42
const y: number = x
```

### any to string

> Any is assignable to string (unsafe but allowed).

```ds
const x: any = "hello"
const y: string = x
```
