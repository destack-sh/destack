# Widening in Match Expressions

Match expressions should use scrutinee types as contextual types for pattern bindings.
Widening should happen at commitment points such as bindings and match expression results.

## Pattern bindings

### match patterns inherit tuple literal precision from const assertions

> Pattern bindings in match arms should reflect tuple precision from const assertions.

```ds
const pair = [1, 2] as const;

match (pair) {
    [first, second] => {
        first satisfies 1;
        second satisfies 2;
    }
}
```

### match patterns widen array elements without const assertions

> Array element bindings should widen without a const context.

```ds
const pair = [1, 2];

match (pair) {
    [first, second] => {
        first satisfies number;
        second satisfies number;
    }
}
```

### match expression results widen in let bindings

> Let bindings should commit match results to widened types when no contextual type exists.

```ds
let value = match (1) {
    1 => 1
    _ => 2
};

value satisfies number;
```

### match expression results do not keep single literals in let bindings

> Let bindings should not retain single literal results from match expressions.

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

> Const bindings should keep literal unions when no widening commitment is required.

```ds
const value = match (1) {
    1 => 1
    _ => 2
};

value satisfies 1 | 2;
```
