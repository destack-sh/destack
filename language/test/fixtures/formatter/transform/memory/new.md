# New Expressions

New expression fixtures cover constructor calls, type arguments, JSX arguments, and member grouping.

## Constructor Calls

### new with empty args

Constructor calls keep tight parentheses.

```ds
const value = new Foo()
```

```ds expected
const value = new Foo();
```

### new with arguments

Constructor arguments follow call formatting rules.

```ds
const value = new Foo(a, b, c)
```

```ds expected
const value = new Foo(a, b, c);
```

## Line Breaking

### new call breaks at line width

Long constructor argument lists expand like normal calls.

```ds line-width=30
const value = new Foo(firstArg, secondArg, thirdArg)
```

```ds expected
const value = new Foo(
    firstArg,
    secondArg,
    thirdArg,
);
```

## Type Arguments

### new with type arguments

Type arguments keep tight spacing.

```ts:main.ts
const value = new Box<Thing>(item)
```

```ts expected
const value = new Box<Thing>(item);
```

## JSX Arguments

### new with jsx argument removes extra parentheses

JSX arguments do not keep extra parentheses.

```tsx:main.tsx
return new ImageResponse(
  (
    <div>
    </div>
  ),
)
```

```tsx expected
return new ImageResponse(<div></div>);
```

## Computed Members

### new with computed member keeps grouping

Computed member expressions keep parentheses for `new`.

```ts:main.ts
new (get(win))[ty](xxx)
```

```ts expected
new (get(win)[ty])(xxx);
```

### new with member expression drops redundant grouping

Simple member expressions drop unnecessary parentheses for `new`.

```ts:main.ts
new (Foo.bar)(value)
```

```ts expected
new Foo.bar(value);
```

### new with chained member call

Chains after `new` stay on the same line when short.

```ts:main.ts
new Foo().bar()
```

```ts expected
new Foo().bar();
```

### new with nested computed members keeps grouping

Computed member chains keep their parentheses for `new`.

```ts:main.ts
new (get(win))[ty][ty](xxx)
```

```ts expected
new (get(win)[ty][ty])(xxx);
```

### new with optional chain computed member keeps grouping

Optional chain computed access keeps its parentheses for `new`.

```ts:main.ts
new (A?.[ty])(xxx)
```

```ts expected
new (A?.[ty])(xxx);
```

### new with optional chain member computed keeps grouping

Optional chain member access stays parenthesized for `new`.

```ts:main.ts
new (A?.B[ty])(xxx)
```

```ts expected
new (A?.B[ty])(xxx);
```

### new with quoted member normalizes quotes

Quoted member keys follow quote style.

```ts:main.ts
new window['TouchEvent'](xxx)
```

```ts expected
new window["TouchEvent"](xxx);
```
