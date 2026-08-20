# New Expressions

New expression fixtures cover constructor calls, type arguments, tree arguments, and member grouping.

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

```ds:main.ds
const value = new Box<Thing>(item)
```

```ds expected
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

## Tree Arguments

### new with tree argument removes extra parentheses

Tree arguments do not keep extra parentheses.

```ds:main.ds
return new ImageResponse(
  (
    <div>
    </div>
  ),
)
```

```ds expected
return new ImageResponse(<div></div>);
```

## Members

### new with member expression

Member constructor names format without grouping.

```ds:main.ds
new (Foo.bar)(value)
```

```ds expected
new Foo.bar(value);
```

### new with chained member call

Chains after `new` stay on the same line when short.

```ds:main.ds
new Foo().bar()
```

```ds expected
new Foo().bar();
```
