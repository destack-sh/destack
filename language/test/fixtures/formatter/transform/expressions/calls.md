# Function Calls

Tests for function call expression formatting.

## Basic Calls

### function calls have no internal spacing

Spaces after `(` and before `)` should be removed.

```ds
foo( a, b, c )
```

```ds expected
foo(a, b, c);
```

### short argument lists stay on one line

Short calls remain on a single line.

```ds
foo(a, b, c)
```

```ds expected
foo(a, b, c);
```

### empty function call

Empty argument lists have all spaces removed.

```ds
foo(   )
```

```ds expected
foo();
```

### single argument call

Single arguments have internal spacing removed.

```ds
foo(   x   )
```

```ds expected
foo(x);
```

## Line Breaking

### call breaks when exceeding line width

When arguments exceed line width, they break to multiple lines.

```ds line-width=20
foo(aLongArg, anotherLongArg, thirdArg)
```

```ds expected
foo(
    aLongArg,
    anotherLongArg,
    thirdArg,
);
```

### call with many short arguments breaks at line width

Many short arguments also break when exceeding line width.

```ds line-width=30
foo(a, b, c, d, e, f, g, h, i, j)
```

```ds expected
foo(
    a,
    b,
    c,
    d,
    e,
    f,
    g,
    h,
    i,
    j,
);
```

### nested function calls

Nested calls are preserved without extra spacing.

```ds
foo(bar(baz(x)))
```

```ds expected
foo(bar(baz(x)));
```

### callback as last argument

Arrow function callbacks stay on one line if short.

```ds
array.map((item) => item.value)
```

```ds expected
array.map((item) => item.value);
```

### callback with body

Block bodies in callbacks expand to multiple lines.

```ds
array.map((item) => { return item.value })
```

```ds expected
array.map((item) => {
    return item.value
});
```

## Method Chains

### short chains stay on one line

Short method chains remain on a single line.

```ds
foo().bar().baz()
```

```ds expected
foo().bar().baz();
```

### long chain breaks at each method

When chains exceed line width, each method gets its own line.

```ds line-width=30
data.filter(x => x.active).map(x => x.name).join(", ")
```

```ds expected
data
    .filter((x) => x.active)
    .map((x) => x.name)
    .join(", ")
;
```

### promise chain

Promise chains break nicely across lines.

```ds line-width=40
fetch(url).then(r => r.json()).then(data => process(data)).catch(handleError)
```

```ds expected
fetch(url)
    .then((r) => r.json())
    .then((data) => process(data))
    .catch(handleError)
;
```

### chain with mixed access

Method and property access can mix in chains.

```ds
obj.items.filter(x => x.valid).length
```

```ds expected
obj.items.filter((x) => x.valid).length;
```

### nested chains

Nested method chains format correctly.

```ds
outer.map(x => x.inner.filter(y => y.ok))
```

```ds expected
outer.map((x) => x.inner.filter((y) => y.ok));
```

## Optional Chaining

### optional method call

Optional chaining uses `?.` for nullable access.

```ds
obj?.method()
```

```ds expected
obj?.method();
```

## Generic Calls

### function call with type arguments

Generic type arguments appear in angle brackets.

```ds
foo<number>(x)
```

```ds expected
foo<number>(x);
```

### function call with multiple type arguments

Multiple type arguments are separated by comma and space.

```ds
foo<number, string, boolean>(x, y, z)
```

```ds expected
foo<number, string, boolean>(x, y, z);
```

### method call with type arguments

Method calls can also have type arguments. Arrow function params get parentheses.

```ds
array.map<string>(x => x.toString())
```

```ds expected
array.map<string>((x) => x.toString());
```

## Binary Expressions

### short binary expressions stay on one line

Short binary expressions remain on a single line.

```ds
a + b + c + d
```

```ds expected
a + b + c + d;
```

### mixed precedence binary expressions

Operator precedence is preserved without added parentheses.

```ds
a + b * c - d / e
```

```ds expected
a + b * c - d / e;
```

## Spread Arguments

### spread in function call

Spread operator expands arrays into arguments.

```ds
foo(...args)
```

```ds expected
foo(...args);
```

### mixed spread and regular arguments

Spread can appear anywhere in the argument list.

```ds
foo(a, b, ...rest, c)
```

```ds expected
foo(a, b, ...rest, c);
```

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

## Tagged Templates

### tagged template literal

Tagged templates apply a function to a template literal.

```ds
sql`SELECT * FROM users`
```

```ds expected
sql`SELECT * FROM users`;
```

### tagged template with interpolation

Tagged templates can include interpolated expressions.

```ds
html`<div>${content}</div>`
```

```ds expected
html`<div>${content}</div>`;
```
