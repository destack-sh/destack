# Function Returns

Function bodies must produce the declared return type.

## returns

### block bodies reject missing returns

> Not all code paths return a value.

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
}
```

- contains: missing return
- contains: not assignable

### tail expressions satisfy return types

> Implicit return expressions satisfy the return requirement.

```ds
function example(value: number): number {
    if (value > 0) {
        return value;
    }
    value + 1
}
```
