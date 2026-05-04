# Static If Statements

Static if gating on statements.

## gating

### static if gates module statements

> Module statements gated by static if are removed before resolution.

```ds
@if(import.meta.emit == "js" && import.meta.emit == "native")
missingSymbol;

const value = 1;
```

### static if keeps module statements when true

> Module statements gated by true static if conditions remain available.

```ds
@if(true)
const value = 1;

value satisfies number;
```

### static if gates block statements

> Block statements gated by static if are removed before resolution.

```ds
function demo(): number {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    missingSymbol;

    return 1;
}
```

### static if keeps block statements when true

> Block statements gated by true static if conditions remain available.

```ds
function demo(): number {
    let value: number = 0;

    @if(true)
    value = 1;

    return value;
}
```

### static if gates block declarations

> Block declarations gated by static if are removed before resolution.

```ds
function compute(): number {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    const hidden = missingSymbol;

    return 1;
}
```
