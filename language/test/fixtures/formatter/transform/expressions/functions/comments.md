# Function Comments

## Comments

### function with doc comment

Doc comments are preserved above the function declaration.

```ds
/// Adds two numbers together.
function add(a: number, b: number): number { return a + b }
```

```ds expected
/// Adds two numbers together.
function add(a: number, b: number): number {
    return a + b;
}
```

### function with inline comment

Inline comments at the start of a block move to their own line.

```ds
function foo() { // inline comment
    return 1
}
```

```ds expected
function foo() {
    // inline comment
    return 1;
}
```

## Comments Causing Expansion
### comment in function body

Comments in function bodies are preserved.

```ds
function foo() { /* empty */ }
```

```ds expected
function foo() {
    /* empty */
}
```
