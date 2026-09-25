# Function Wrapping

## Line Breaking

### function with many parameters breaks

When parameters exceed the line width, they break to multiple lines with trailing comma.

```tspp line-width=40
function foo(veryLongParam: string, anotherLongParam: number, thirdParam: boolean) { }
```

```tspp expected
function foo(
    veryLongParam: string,
    anotherLongParam: number,
    thirdParam: boolean,
) {}
```

### generic function with many type params breaks

Type parameters also break when they exceed the line width.

```tspp line-width=40
function foo<VeryLongType, AnotherLongType, ThirdType>(x: VeryLongType): void { }
```

```tspp expected
function foo<
    VeryLongType,
    AnotherLongType,
    ThirdType,
>(x: VeryLongType): void {}
```

### function with where clause

Where clauses specify additional type constraints.

```tspp
function process<T>(x: T): T where T: Copy { return x }
```

```tspp expected
function process<T>(x: T): T where T: Copy {
    return x;
}
```

### function with multiple where constraints

Multiple where constraints print without grouping parentheses when they fit.

```tspp line-width=50
function process<T, U>(a: T, b: U): void where (T: Copy, U: Clone) { }
```

When the signature is too long, the where clause breaks as an indented continuation.

```tspp expected
function process<T, U>(a: T, b: U): void
    where T: Copy, U: Clone {}
```

### function with long where constraints

Long where constraints break under the `where` keyword.

```tspp line-width=72
function resolve<T, U, V>(value: T): V where T: VeryLongCopyConstraint, U: VeryLongCloneConstraint, V: VeryLongComparableConstraint { return value }
```

```tspp expected
function resolve<T, U, V>(value: T): V
    where
        T: VeryLongCopyConstraint,
        U: VeryLongCloneConstraint,
        V: VeryLongComparableConstraint {
    return value;
}
```
