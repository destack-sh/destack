# Code Actions

Tests for LSP code actions functionality.

## Basic Tests

### No code actions for valid code

Valid code without errors should have no code actions.

```ds
function greet(name: string): string {
    return "Hello, " + name;
}

const msg = greet("World");
```

No code actions expected for valid code.

```query code_actions $0
<none>
```

### No code actions for empty file

Empty or minimal files should not generate code actions.

```ds
const x = 42;
```

No code actions expected.

```query code_actions $0
<none>
```
