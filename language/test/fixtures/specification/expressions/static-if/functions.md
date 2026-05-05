# Static If Functions

Functions can be gated with `@if`.

## gating

### when false, `@if` removes function declarations

When false, `@if` removes function declarations.

```ds
@if(false)
function hidden(): MissingType {
    return missingSymbol;
}

function visible(): number {
    return 1;
}
```

### when true, `@if` includes function declarations

When true, `@if` includes function declarations.

```ds
@if(true)
function add(left: number, right: number): number {
    return left + right;
}

const total = add(1, 2);
total satisfies number;
```
