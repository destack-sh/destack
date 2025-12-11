# Range Literals

> NOTE #Incomplete: implement/mdtest range literals

Range expressions for iteration and slicing.

## Coverage

- **Exclusive ranges**: `0..10` produces [0, 10)
- **Inclusive ranges**: `0..=10` produces [0, 10]
- **Variable ranges**: `start..end`
- **Iteration**: `for (const i of 0..10)`

## Example

```ds
for (const i of 0..10) {
    print(i);  // 0, 1, 2, ..., 9
}

for (const i of 0..=10) {
    print(i);  // 0, 1, 2, ..., 10
}
```
