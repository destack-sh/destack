# Loop Expression

> NOTE #Incomplete: implement/mdtest loop expression

Infinite loop construct.

## Coverage

- **Infinite loop**: `loop { ... }`
- **Break**: Exit with `break`
- **Break with value**: `break value` (loop as expression)
- **Labeled loops**: `outer: loop { break outer }`

## Example

```ds
loop {
    const input = readInput();
    if (input == "quit") {
        break;
    }
    process(input);
}

// loop as expression
const result = loop {
    const value = compute();
    if (value > threshold) {
        break value;
    }
};
```
