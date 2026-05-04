# Widening in Match Expressions

Match expressions use scrutinee types as contextual types for pattern bindings.
Widening happens when values are bound or when match expression results are joined.

## Pattern bindings

### match patterns inherit tuple literal precision from const assertions

> Pattern bindings in match arms reflect tuple precision from const assertions.

```ds
const pair = [1, 2] as const;

match (pair) {
    [first, second] => {
        first satisfies 1;
        second satisfies 2;
    }
}
```

### match patterns widen array element types without const context

> Array element bindings widen without a const context.
> A fallback arm is required because non fixed arrays are not exhaustively covered by a single pattern.

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

### match patterns are exhaustive for fixed arrays

> Fixed-size arrays can be exhaustively matched without a fallback arm.

```ds
declare const pair: [int32; 2];

match (pair) {
    [first, second] => {
        first satisfies int32;
        second satisfies int32;
    }
}
```

### match expression results widen in let bindings

> Let bindings commit match results to widened types when no contextual type exists.

```ds
let value = match (1) {
    1 => 1
    _ => 2
};

value satisfies number;
```

### match expression results widen without fallback for literal scrutinees

> Exhaustive literal matches still widen at let bindings.

```ds
let value = match (1) {
    1 => 1
};

value satisfies number;
```

### match expression results widen fresh literals in let bindings

> Let bindings widen fresh literal results to their primitive types.

```ds
let value = match (1) {
    1 => 1
    _ => 2
};

value satisfies 1;
```

- contains: not assignable

## Match results

### match expression results keep literal unions in const bindings

> Const bindings keep literal unions when no widening commitment is required.

```ds
const value = match (1) {
    1 => 1
    _ => 2
};

value satisfies 1 | 2;
```
