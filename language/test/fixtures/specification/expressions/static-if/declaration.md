# Static If Declarations

Static if gating on declarations.

## Gating

### static if gates const declarations

> Declarations with false static if conditions are removed before resolution.

```ds
@if(import.meta.emit == "js" && import.meta.emit == "native")
const hidden = missingSymbol;

const visible = 1;
```

### static if keeps const declarations when true

> Declarations gated by true static if conditions remain available.

```ds
@if(true)
const value = 1;

value satisfies number;
```

### static if gates function declarations

> Gated function declarations are removed before resolution.

```ds
@if(import.meta.emit == "js" && import.meta.emit == "native")
function hidden(): MissingType {
    return missingSymbol;
}

function visible(): number {
    return 1;
}
```

### static if keeps function declarations when true

> Function declarations gated by true static if conditions remain available.

```ds
@if(true)
function add(left: number, right: number): number {
    return left + right;
}

const total = add(1, 2);
total satisfies number;
```

### static if gates type aliases

> Gated type aliases are removed before resolution.

```ds
@if(import.meta.emit == "js" && import.meta.emit == "native")
type Hidden = MissingType;

type Visible = number;
```

### static if keeps type aliases when true

> Type aliases gated by true static if conditions remain available.

```ds
@if(true)
type Visible = number;

declare const value: Visible;
value satisfies number;
```

### static if combines multiple annotations

> Multiple @if annotations combine with logical and semantics.

```ds
@if(import.meta.emit == "js")
@if(import.meta.emit == "native")
const combined = missingSymbol;
```
