# Block Expressions

> NOTE #Incomplete: implement/mdtest block expressions

Blocks as expressions with implicit returns.

## Coverage

- **Block value**: Last expression (no `;`) is the block's value
- **Labeled blocks**: `label: { ... break label value }`
- **Do blocks**: `do { ... }` for disambiguation in JSX

## Example

```ds
const result = {
    const x = compute();
    const y = transform(x);
    x + y  // implicit return
};

const value = outer: {
    for (const i of 0..100) {
        if (condition(i)) {
            break outer i;
        }
    }
    -1  // default
};

// do block in JSX context
<Component value={do { let x = prepare(); transform(x) }} />
```
