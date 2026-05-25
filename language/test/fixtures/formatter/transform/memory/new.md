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

### new with inferred type

The inferred constructor marker formats like a type name.

```ds
const value = new _ ( item )
```

```ds expected
const value = new _(item);
```

### new with inferred type argument

Generic constructor arguments may contain inferred type holes.

```ds
const value = new Box < _ > ( item )
```

```ds expected
const value = new Box<_>(item);
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

## Members

### new with member expression

Member constructor names format without grouping.

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

### new with quoted member normalizes quotes

Quoted member keys follow quote style.

```ts:main.ts
new window['TouchEvent'](xxx)
```

```ts expected
new window["TouchEvent"](xxx);
```
