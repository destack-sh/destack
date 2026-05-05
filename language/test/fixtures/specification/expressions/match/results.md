# Match Results

Match expressions type their patterns from the matched value.
Arm bodies then join like other expression branches.

## pattern bindings

### const tuples keep literal elements

> Const tuple patterns bind literal elements.

```ds
const pair = [1, 2] as const;

match (pair) {
    [first, second] => {
        first satisfies 1;
        second satisfies 2;
    }
}
```

### plain arrays widen elements

> Array element bindings widen without a const context.
> Open arrays need a fallback arm.

```ds
const pair = [1, 2];

match (pair) {
    [first, second] => {
        first satisfies number;
        second satisfies number;
    }
    _ => {}
}
```

### fixed arrays are exhaustive by length

> Fixed array patterns cover every element position.

```ds
declare const pair: [int32; 2];

match (pair) {
    [first, second] => {
        first satisfies int32;
        second satisfies int32;
    }
}
```

### let bindings widen match results

> Let bindings widen match results without an annotation.

```ds
let value = match (1) {
    1 => 1
    _ => 2
};

value satisfies number;
```

### literal scrutinees still use binding widening

> Exhaustive literal matches still widen at let bindings.

```ds
let value = match (1) {
    1 => 1
};

value satisfies number;
```

### fresh literal results widen in let bindings

> Let bindings widen fresh literal results to their primitive types.

```ds
let value = match (1) {
    1 => 1
    _ => 2
};

value satisfies 1;
```

- contains: not assignable

## match results

### result annotations check arms

> Each arm must satisfy the annotated result type.

```ds
function choose(value: int32): string {
    const result: string = match (value) {
        0 => "zero"
        _ => 1
    };
    result
}
```

- contains: not assignable

### const bindings keep literal unions

> Const bindings keep literal unions.

```ds
const value = match (1) {
    1 => 1
    _ => 2
};

value satisfies 1 | 2;
```
