# Generic Functions

## Generic Functions

### generic function

Type parameters appear in angle brackets after the function name.

```ds
function identity<T>(x: T): T { return x }
```

```ds expected
function identity<T>(x: T): T {
    return x;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```ds
function process<T: Comparable>(a: T, b: T): boolean { return a < b }
```

```ds expected
function process<T: Comparable>(a: T, b: T): boolean {
    return a < b;
}
```

### multiple type parameters

Multiple type parameters are separated by commas with no trailing comma.

```ds
function merge<T, U>(a: T, b: U): T & U { return { ...a, ...b } }
```

```ds expected
function merge<T, U>(a: T, b: U): T & U {
    return { ...a, ...b };
}
```

### generic with default

Default type parameters use `= Type` syntax.

```ds
function create<T = any>(): T[] { return [] }
```

```ds expected
function create<T = any>(): T[] {
    return [];
}
```

### generic with comptime parameter

Comptime static parameters keep the keyword in the parameter list.

```ds
function repeat<comptime N: int>(value: string): string { return value }
```

```ds expected
function repeat<comptime N: int>(value: string): string {
    return value;
}
```
