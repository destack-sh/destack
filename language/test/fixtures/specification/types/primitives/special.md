# Special Literals

Null, undefined, and other special literals.

## null

### null literal

> Null can be assigned to null type.

```ds
const x: null = null;
x satisfies null;
```

### null rejects non-nullable targets

> Null is not assignable to non-nullable types.

```ds
const value: string = null;
```

- contains: not assignable

## undefined

### undefined literal

> Undefined can be assigned to undefined type.

```ds
const x: undefined = undefined;
x satisfies undefined;
```

## void

### void type

> Functions can have void return type for no return value.

```ds
function f(): void {}
```
