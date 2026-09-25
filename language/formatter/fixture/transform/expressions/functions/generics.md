# Generic Functions

## Generic Functions

### generic function

Type parameters appear in angle brackets after the function name.

```tspp
function identity<T>(x: T): T { return x }
```

```tspp expected
function identity<T>(x: T): T {
    return x;
}
```

### generic with constraint

Type constraints use colon syntax: `T: Constraint`.

```tspp
function process<T: Comparable>(a: T, b: T): boolean { return a < b }
```

```tspp expected
function process<T: Comparable>(a: T, b: T): boolean {
    return a < b;
}
```

### multiple type parameters

Multiple type parameters are separated by commas with no trailing comma.

```tspp
function merge<T, U>(a: T, b: U): T & U { return { ...a, ...b } }
```

```tspp expected
function merge<T, U>(a: T, b: U): T & U {
    return { ...a, ...b };
}
```

### generic with default

Default type parameters use `= Type` syntax.

```tspp
function create<T = any>(): T[] { return [] }
```

```tspp expected
function create<T = any>(): T[] {
    return [];
}
```

### generic with const parameter

Const parameters keep the keyword in the parameter list.

```tspp
function repeat<const N: int>(value: string): string { return value }
```

```tspp expected
function repeat<const N: int>(value: string): string {
    return value;
}
```

### generic with equality predicate

Where-clause equality predicates keep the equality operator.

```tspp
function project<T,U>():T where T.Output==U { return value }
```

```tspp expected
function project<T, U>(): T where T.Output == U {
    return value;
}
```
