# Static If Validation

Tests for static if validation errors.

## Diagnostics

### static if errors when true

> Static if conditions that evaluate to true are enforced.

```ds
@if(true)
missingSymbol;
```

- contains: missing symbol

### static if requires boolean

> Static if conditions must resolve to boolean values.

```ds
@if(1)
const value = 1;
```

- contains: static if condition must be boolean

### static if requires an argument

> Static if decorators require exactly one argument.

```ds
@if
const value = 1;
```

- contains: static if requires a condition argument

### static if rejects extra arguments

> Static if decorators accept a single argument.

```ds
@if(true, false)
const value = 1;
```

- contains: static if requires exactly one argument
