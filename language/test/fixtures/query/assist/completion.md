# Completion

## Basic Symbols

### Complete with available keywords

At any position in the file, completion should offer language keywords.

```ds
const foo = 1;
$0
```

Common keywords like `function`, `const`, and `struct` should appear in the completion list.

```query completion $0
- function: keyword
- const: keyword
- struct: keyword
```

### Complete with keywords at start

Even in an empty file, completion should offer language keywords.

```ds
$0
```

This verifies the baseline completion functionality works without any context.

```query completion $0
- function: keyword
- const: keyword
```
