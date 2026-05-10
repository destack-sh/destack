# Function Wrapping

## Line Breaking

### function with many parameters breaks

When parameters exceed the line width, they break to multiple lines with trailing comma.

```ds line-width=40
function foo(veryLongParam: string, anotherLongParam: number, thirdParam: boolean) { }
```

```ds expected
function foo(
    veryLongParam: string,
    anotherLongParam: number,
    thirdParam: boolean,
) {}
```

### generic function with many type params breaks

Type parameters also break when they exceed the line width.

```ds line-width=40
function foo<VeryLongType, AnotherLongType, ThirdType>(x: VeryLongType): void { }
```

```ds expected
function foo<
    VeryLongType,
    AnotherLongType,
    ThirdType,
>(x: VeryLongType): void {}
```

### function with where clause

Where clauses specify additional type constraints.

```ds
function process<T>(x: T): T where T: Copy { return x }
```

```ds expected
function process<T>(x: T): T where T: Copy {
    return x;
}
```

### function with multiple where constraints

Multiple where constraints can be grouped in parentheses.

```ds line-width=50
function process<T, U>(a: T, b: U): void where (T: Copy, U: Clone) { }
```

When the signature is too long, the where clause breaks to its own line.

```ds expected
function process<T, U>(a: T, b: U): void
where (T: Copy, U: Clone) {}
```
