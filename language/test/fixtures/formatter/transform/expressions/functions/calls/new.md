# New Calls

## New Expressions

### new with arguments

Constructor calls use `new` followed by the class name.

```ds
new Foo(a, b, c)
```

```ds expected
new Foo(a, b, c);
```

### new without arguments

Empty parentheses are preserved on `new` expressions.

```ds
new Foo()
```

```ds expected
new Foo();
```

### chained new expression

Method chains can follow `new` expressions.

```ds
new Foo().bar().baz()
```

```ds expected
new Foo().bar().baz();
```
