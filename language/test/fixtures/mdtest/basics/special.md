# Special Literals

Tests for null, undefined, and other special literals.

## Null

### null literal

> Null can be assigned to null type.

```ds
const x: null = null;
x satisfies null;
```

## Undefined

### undefined literal

> Undefined can be assigned to undefined type.

```ds
const x: undefined = undefined;
x satisfies undefined;
```

## Void

### void type

> Functions can have void return type for no return value.

```ds
function f(): void {}
```
