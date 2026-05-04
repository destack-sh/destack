# Static If Validation

Static if validation errors.

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

> Static if annotations require exactly one argument.

```ds
@if
const value = 1;
```

- contains: static if requires a condition argument

### static if rejects extra arguments

> Static if annotations accept a single argument.

```ds
@if(true, false)
const value = 1;
```

- contains: static if requires exactly one argument

### static if rejects parameter placement

> Static if annotations are not allowed on parameters.

```ds
function demo(@if(true) value: number): void { }
```

- invalid static if: static if is only allowed on declarations, members, enum fields, or statements

### static if rejects type literal properties

> Static if annotations are not allowed on type literal properties.

```ds
type Box = {
    @if(true)
    value: number;
};
```

- invalid static if: static if is only allowed on declarations, members, enum fields, or statements

### static if rejects argument placement

> Static if annotations are not allowed on call arguments.

```ds
function call(value: number): void { }

call(@if(true) 1);
```

- invalid static if: static if is only allowed on declarations, members, enum fields, or statements
