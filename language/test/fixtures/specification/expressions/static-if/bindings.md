# Static If Bindings

Bindings can be gated with `@if`.

## gating

### when false, `@if` removes const declarations

When false, `@if` removes const declarations.

```ds
@if(false)
const hidden = missingSymbol;

const visible = 1;
```

### when true, `@if` includes const declarations

When true, `@if` includes const declarations.

```ds
@if(true)
const value = 1;

value satisfies number;
```
