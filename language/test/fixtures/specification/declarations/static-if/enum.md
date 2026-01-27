# Static If Enums

Tests for static if gating on enum fields.

## Gating

### static if gates enum fields

> Enum fields gated by static if are removed before resolution.

```ds
enum Status {
    @if(import.meta.output == "js" && import.meta.output == "native")
    Missing = missingSymbol,
    Visible = 1,
}
```

### static if keeps enum fields when true

> Enum fields gated by true static if conditions remain available.

```ds
enum Status {
    @if(true)
    Active = 1,
    Inactive = 2,
}

const active = Status.Active;
active satisfies Status;
```
