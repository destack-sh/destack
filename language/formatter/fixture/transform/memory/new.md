# New Expressions

New expression fixtures cover constructor calls, type arguments, tree arguments, and member grouping.

## Constructor Calls

### new with empty args

Constructor calls keep tight parentheses.

```tspp
const value = new Foo()
```

```tspp expected
const value = new Foo();
```

### new with arguments

Constructor arguments follow call formatting rules.

```tspp
const value = new Foo(a, b, c)
```

```tspp expected
const value = new Foo(a, b, c);
```

## Line Breaking

### new call breaks at line width

Long constructor argument lists expand like normal calls.

```tspp line-width=30
const value = new Foo(firstArg, secondArg, thirdArg)
```

```tspp expected
const value = new Foo(
    firstArg,
    secondArg,
    thirdArg,
);
```

## Type Arguments

### new with type arguments

Type arguments keep tight spacing.

```tspp:main.tspp
const value = new Box<Thing>(item)
```

```tspp expected
const value = new Box<Thing>(item);
```

### new with inferred type

The inferred constructor marker formats like a type name.

```tspp
const value = new _ ( item )
```

```tspp expected
const value = new _(item);
```

### new with inferred type argument

Generic constructor arguments may contain inferred type holes.

```tspp
const value = new Box < _ > ( item )
```

```tspp expected
const value = new Box<_>(item);
```

## Tree Arguments

### new with tree argument removes extra parentheses

Tree arguments do not keep extra parentheses.

```tspp:main.tspp
return new ImageResponse(
  (
    <div>
    </div>
  ),
)
```

```tspp expected
return new ImageResponse(<div></div>);
```

## Members

### new with member expression

Member constructor names format without grouping.

```tspp:main.tspp
new (Foo.bar)(value)
```

```tspp expected
new Foo.bar(value);
```

### new with chained member call

Chains after `new` stay on the same line when short.

```tspp:main.tspp
new Foo().bar()
```

```tspp expected
new Foo().bar();
```

## Constructor Expressions

### Indexed constructors

Indexing selects the constructor before its arguments.

```tspp
new constructors [ 0 ] ( value )
new constructors [ 0 ] < Item > ( value ).member
```

```tspp expected
new constructors[0](value);
new constructors[0]<Item>(value).member;
```

### Constructors returned by calls

Parentheses separate the call that returns a constructor from the construction.

```tspp
new ( selectConstructor ( ) ) ( value )
new ( selectConstructor ( ).member ) ( value )
```

```tspp expected
new (selectConstructor())(value);
new (selectConstructor().member)(value);
```

### Conditional constructors

Parentheses retain the complete conditional operand.

```tspp
new ( enabled ? Primary : Secondary ) ( value )
```

```tspp expected
new (enabled ? Primary : Secondary)(value);
```
