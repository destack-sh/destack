# Static If Declarations

Tests for static if gating on declarations.

## Gating

### static if gates const declarations

> Declarations with false static if conditions are removed before resolution.

```ds
@if(import.meta.output == "js" && import.meta.output == "native")
const hidden = missingSymbol;

const visible = 1;
```

### static if gates function declarations

> Gated function declarations are removed before resolution.

```ds
@if(import.meta.output == "js" && import.meta.output == "native")
function hidden(): MissingType {
    return missingSymbol;
}

function visible(): number {
    return 1;
}
```

### static if gates type aliases

> Gated type aliases are removed before resolution.

```ds
@if(import.meta.output == "js" && import.meta.output == "native")
type Hidden = MissingType;

type Visible = number;
```

### static if combines multiple decorators

> Multiple @if decorators combine with logical and semantics.

```ds
@if(import.meta.output == "js")
@if(import.meta.output == "native")
const combined = missingSymbol;
```
