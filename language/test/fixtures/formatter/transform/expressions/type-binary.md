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

### angle bracket assertion

Angle bracket assertions preserve `<T>expr` formatting.

```ts:main.ts
const value = <Foo>bar
```

```ts expected
const value = <Foo>bar;
```

### angle bracket assertion with binary value

Angle bracket assertions add parentheses when needed to preserve meaning.

```ts:main.ts
const value = <Foo>a + b
```

```ts expected
const value = <Foo>(a + b);
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
