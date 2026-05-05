# Static If Validation

Invalid `@if` forms are rejected.

## rejections

### when true, `@if` includes errors

When true, `@if` includes the annotated statement.

```ds
@if(true)
missingSymbol;
```

- contains: missing symbol

### @if requires boolean conditions

`@if` conditions must resolve to boolean values.

```ds
@if(1)
const value = 1;
```

- contains: static if condition must be boolean

### @if requires static conditions

`@if` conditions cannot depend on runtime values.

```ds
declare const enabled: bool;

@if(enabled)
const value = 1;
```

- contains: static if condition must be static

### @if requires a condition

`@if` requires exactly one condition argument.

```ds
@if
const value = 1;
```

- contains: static if requires a condition argument

### @if rejects extra arguments

`@if` accepts one condition argument.

```ds
@if(true, false)
const value = 1;
```

- contains: static if requires exactly one argument

### @if rejects parameters

`@if` is not allowed on parameters.

```ds
function demo(@if(true) value: number): void { }
```

- contains: invalid static if: static if is only allowed on declarations, members, enum fields, or statements

### @if rejects type literal properties

`@if` is not allowed on type literal properties.

```ds
type Box = {
    @if(true)
    value: number;
};
```

- contains: invalid static if: static if is only allowed on declarations, members, enum fields, or statements

### @if rejects call arguments

`@if` is not allowed on call arguments.

```ds
function call(value: number): void { }

call(@if(true) 1);
```

- contains: invalid static if: static if is only allowed on declarations, members, enum fields, or statements
