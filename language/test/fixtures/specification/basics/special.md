# Special Literals

Tests for null, undefined, and other special literals.

## Null

### null literal

> Null can be assigned to null type.

```ds
const x: null = null;
x satisfies null;
```

### strictNullChecks rejects null assignments

> Null is not assignable to non-nullable types when strict null checks are enabled.

```ds:dsconfig.json
{ "compilerOptions": { "strictNullChecks": true } }
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

### strictNullChecks allows null assignments when disabled

> Null becomes assignable to other types when strict null checks are disabled.

```ds:dsconfig.json
{ "compilerOptions": { "strictNullChecks": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const value: string = null;
```

## Void

### void type

> Functions can have void return type for no return value.

```ds
function f(): void {}
```
