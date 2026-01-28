# New Expressions

Tests for `new` expression formatting.

## Basic New

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

### new with type arguments (TypeScript)

Type arguments keep tight spacing.

```ts:main.ts
const value = new Box<Thing>(item)
```

```ts expected
const value = new Box<Thing>(item);
```
