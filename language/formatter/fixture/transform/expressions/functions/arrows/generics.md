# Generic Arrow Functions

## Generic Arrow Functions

### arrow function with type parameter

```tspp
const identity = <T,>(x: T): T => x
```

```tspp expected
const identity = <T,>(x: T): T => x;
```

### arrow function with constrained type parameter

```tspp
const first = <T: Iterable<U>, U>(items: T): U => items[0]
```

```tspp expected
const first = <T: Iterable<U>, U>(items: T): U => items[0];
```

### arrow function with multiple type parameters

Multiple generic type parameters in function type syntax.

```tspp
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn)
```

```tspp expected
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn);
```
