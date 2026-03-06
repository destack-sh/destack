# Inlay Hint

## Function Calls

### Show type and parameter hints

Inlay hints should show the inferred binding type and parameter names for literal arguments.

```ds:main.ds
function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}

const msg = greet("World", "Hello");
```

```lsp inlay_hint
position=4:9
label=: string
kind=type
padding_left=false
padding_right=false

position=4:18
label=name:
kind=parameter
padding_left=false
padding_right=true

position=4:27
label=greeting:
kind=parameter
padding_left=false
padding_right=true
```
