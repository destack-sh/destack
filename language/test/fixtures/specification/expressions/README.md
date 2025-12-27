# Expressions

> NOTE #Incomplete: implement/mdtest Destack expression extensions

Expression extensions beyond standard TypeScript.

In TypeScript, `if` is a statement and you need a ternary or temporary to get a value.
In Destack, everything is an expression. The last non-statement expression becomes the value.

## Subdirectories

| Directory | Description |
|-----------|-------------|
| `match/` | Pattern matching with exhaustiveness checking |
| `ranges/` | Range literals (`0..10`, `0..=10`) |
| `tuples/` | Tuple literals and types `(a, b)` |
| `loops/` | Infinite `loop { }` construct |
| `blocks/` | Block expressions, labeled blocks, `do { }` |
| `patterns/` | Pattern syntax for destructuring |

## Coverage

- **Implicit returns**: Last expression is the return value
- **If expressions**: `if` as an expression returning a value

## Example

```ds
const result = if (condition) { computeA() } else { computeB() };

const label = match (state) {
    Ready => "go"
    Loading => "wait"
    Error(e) => `failed: ${e}`
};

for (const i of 0..10) { print(i) }
```

See [DESIGN.md](../../../../../DESIGN.md#expressions) for full documentation.
