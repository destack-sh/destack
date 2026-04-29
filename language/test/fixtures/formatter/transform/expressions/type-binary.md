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

### casted comparison in logical expression

Comparison casts inside logical expressions keep grouping parentheses.

```ts:main.ts
const ok = i < 0 || i >= length as number
```

```ts expected
const ok = i < 0 || ((i >= length) as number);
```

### cast before arithmetic

Casts on the left side of arithmetic stay grouped.

```ts:main.ts
const last = length as number - 1
```

```ts expected
const last = (length as number) - 1;
```

### satisfies before arithmetic

Satisfies expressions on the left side of arithmetic stay grouped.

```ts:main.ts
const last = (length satisfies number) - 1
```

```ts expected
const last = (length satisfies number) - 1;
```

### cast as call callee

Cast expressions used as call callees keep grouping parentheses.

```ts:main.ts
const value = (value as Fn)()
```

```ts expected
const value = (value as Fn)();
```

### cast as member object

Cast expressions used as member objects keep grouping parentheses.

```ts:main.ts
const value = (value as Box).property
```

```ts expected
const value = (value as Box).property;
```

### chained assertions stay direct

Nested assertion chains stay direct when no parent context needs grouping.

```ts:main.ts
const value = input as unknown as Result
```

```ts expected
const value = input as unknown as Result;
```

### assertion in ternary test

Assertions in ternary tests keep grouping parentheses.

```ts:main.ts
const value = (input as boolean) ? yes : no
```

```ts expected
const value = (input as boolean) ? yes : no;
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
