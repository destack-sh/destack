# Type Binary Operators

Tests for TypeScript type binary operators like `as` and `satisfies`.

## Basic Casts

### as cast expression

Cast operators keep spaces around `as`.

```ts:main.ts
const value = input as Foo
```

```ts expected
const value = input as Foo;
```

### satisfies expression

Satisfies operators keep spaces around `satisfies`.

```ts:main.ts
const value = input satisfies Foo
```

```ts expected
const value = input satisfies Foo;
```

## Nested Type Binaries

### call with type binary arguments

Type binary arguments stay grouped in calls.

```ts:main.ts
const value = call(input as Foo, other satisfies Bar)
```

```ts expected
const value = call(input as Foo, other satisfies Bar);
```

## Runtime Guards

### runtime type guard comments

Runtime type guard comments stay on the side of the operator they describe.

```ds
const ok = value /* checked value */ is /* expected type */ string
```

```ds expected
const ok = value /* checked value */ is /* expected type */ string;
```
