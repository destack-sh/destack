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
function demo(@if(true) value: number): void {}
```

- contains: invalid static if: static if is not allowed on function parameters

### @if rejects generic parameters

`@if` is not allowed on generic parameters.

```ds
function demo<@if(true) T>(value: T): void {}
```

- contains: invalid static if: static if is not allowed on generic parameters

### @if rejects required expression operands

`@if` cannot remove a required expression operand.

```ds
const value = 1 + @if(true) 2;
```

- contains: invalid static if: static if cannot remove a required expression operand
