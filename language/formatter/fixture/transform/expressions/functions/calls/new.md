# New Calls

## New Expressions

### new with arguments

Constructor calls use `new` followed by the class name.

```tspp
new Foo(a, b, c)
```

```tspp expected
new Foo(a, b, c);
```

### new without arguments

Empty parentheses are preserved on `new` expressions.

```tspp
new Foo()
```

```tspp expected
new Foo();
```

### chained new expression

Method chains can follow `new` expressions.

```tspp
new Foo().bar().baz()
```

```tspp expected
new Foo().bar().baz();
```
