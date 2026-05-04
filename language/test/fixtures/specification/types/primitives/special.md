# Special Literals

Null, undefined, and other special literals.

## Null

### null literal

> Null can be assigned to null type.

```ds
const x: null = null;
x satisfies null;
```

### strictNullChecks rejects null assignments

> Null is not assignable to non-nullable types when strict null checks are enabled.

```json:destack.json
{ "compiler": { "strictNullChecks": true } }
```

```ds
const value: string = null;
```

- contains: not assignable

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
