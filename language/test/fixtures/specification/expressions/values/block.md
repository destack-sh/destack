# Blocks

Blocks can produce values from their final expression.

## expressions

### if blocks yield branch values

If expressions join the values produced by their branches.

```ds
const enabled = true;
const value = if (enabled) {
    1
} else {
    2
};

value satisfies int32;
```

### blocks without tail expressions yield void

A block with no final expression has type void.

```ds
const enabled = true;
const value: int32 = if (enabled) {
    let item = 1;
} else {
    let item = 2;
};
```

- contains: not assignable

## returns

### function bodies return tail expressions

A function body can return its final expression without `return`.

```ds
function add(left: int32, right: int32): int32 {
    left + right
}

add(1, 2) satisfies int32;
```
