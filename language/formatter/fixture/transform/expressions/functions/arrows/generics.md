# Generic Arrow Functions

## Generic Arrow Functions

### arrow function with type parameter

```ds
const identity = <T,>(x: T): T => x
```

```ds expected
const identity = <T,>(x: T): T => x;
```

### arrow function with constrained type parameter

```ds
const first = <T: Iterable<U>, U>(items: T): U => items[0]
```

```ds expected
const first = <T: Iterable<U>, U>(items: T): U => items[0];
```

### arrow function with multiple type parameters

Multiple generic type parameters in function type syntax.

```ds
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn)
```

```ds expected
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn);
```
