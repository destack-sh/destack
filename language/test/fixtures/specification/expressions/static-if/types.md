# Static If Types

Type aliases can be gated with `@if`.

## gating

### when false, `@if` removes type aliases

When false, `@if` removes type aliases.

```ds
@if(false)
type Hidden = MissingType;

type Visible = number;
```

### when true, `@if` includes type aliases

When true, `@if` includes type aliases.

```ds
@if(true)
type Visible = number;

declare const value: Visible;
value satisfies number;
```
