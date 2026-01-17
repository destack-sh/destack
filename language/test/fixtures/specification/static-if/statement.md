# Static If Statements

Tests for static if gating on statements.

## Gating

### static if gates module statements

> Module statements gated by static if are removed before resolution.

```ds
@if(import.meta.output == "js" && import.meta.output == "native")
missingSymbol;

const value = 1;
```

### static if gates block statements

> Block statements gated by static if are removed before resolution.

```ds
function demo(): number {
    @if(import.meta.output == "js" && import.meta.output == "native")
    missingSymbol;

    return 1;
}
```

### static if gates block declarations

> Block declarations gated by static if are removed before resolution.

```ds
function compute(): number {
    @if(import.meta.output == "js" && import.meta.output == "native")
    const hidden = missingSymbol;

    return 1;
}
```
