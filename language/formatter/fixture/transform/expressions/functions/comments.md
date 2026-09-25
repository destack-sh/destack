# Function Comments

## Comments

### function with doc comment

Doc comments are preserved above the function declaration.

```tspp
/// Adds two numbers together.
function add(a: number, b: number): number { return a + b }
```

```tspp expected
/// Adds two numbers together.
function add(a: number, b: number): number {
    return a + b;
}
```

### function with inline comment

Inline comments at the start of a block move to their own line.

```tspp
function foo() { // inline comment
    return 1
}
```

```tspp expected
function foo() {
    // inline comment
    return 1;
}
```

## Comments Causing Expansion
### comment in function body

Comments in function bodies are preserved.

```tspp
function foo() { /* empty */ }
```

```tspp expected
function foo() {
    /* empty */
}
```
