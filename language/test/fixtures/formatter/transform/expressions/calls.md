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

### blank lines between callee and arguments

Blank lines between the callee and argument list are removed.

```ts:main.ts
gen

    ("a");

gen

    (

      "b");
```

```ts expected
gen("a");

gen("b");
```

### single argument call

Single arguments have internal spacing removed.

```ds
foo(   x   )
```

```ds expected
foo(x);
```

### call on instantiation expression

Calls on parenthesized instantiation expressions keep the parentheses.

```ts:main.ts
const value = (makeFactory<number>)(config)
```

```ts expected
const value = makeFactory<number>(config);
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
    return item.value;
});
```

## TypeScript Call Grouping

### multiple arrow arguments break

Arrow callbacks break to multiple lines when there are several.

```ts:main.ts
call(() => foo, () => bar)
```

```ts expected
call(
    () => foo,
    () => bar,
);
```

### block callback with extra arguments breaks

Block-bodied callbacks expand when paired with other arguments.

```ts:main.ts
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1)
)
```

```ts expected
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1),
);
```

### blank lines between arguments

Blank lines between arguments are preserved.

```ts:main.ts
call(
  () => {
    // ...
  },

  "good"
)
```

```ts expected
call(
    () => {
        // ...
    },

    "good",
);
```

### trailing comment on last argument

Trailing comments stay attached to their argument.

```ts:main.ts
call(
  () => {
    // ...
  },
  "good" // trailing
)
```

```ts expected
call(
    () => {
        // ...
    },
    "good", // trailing
);
```

### template literal argument

Template literal arguments keep their indentation.

```ts:main.ts
expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `)
```

```ts expected
expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot(`
  `);
```

### optional call boundary line comment

Line comments between a callee and optional call stay on the full call expression.

```ts:main.ts
call // C4
?.()
```

```ts expected
call?.(); // C4
```

### optional call separator block comment

Block comments between the callee and `?.` stay before the optional operator.

```ts:main.ts
alert /* comment */?.("value")
```

```ts expected
alert /* comment */?.("value");
```

### optional call with inline comment argument

Inline block comments in empty optional call arguments stay inside `()`.

```ts:main.ts
call?.(/* argument comment */)
```

```ts expected
call?.(/* argument comment */);
```

### empty call with line comment argument

Line comments in empty call arguments stay inside multiline `()`.

```ts:main.ts
call(
  // argument line comment
)
```

```ts expected
call(
    // argument line comment
);
```

### empty optional call with line comment argument

Line comments in empty optional call arguments stay inside multiline `()`.

```ts:main.ts
call?.( // argument line comment
)
```

```ts expected
call?.(
    // argument line comment
);
```

### multiple function expressions break

Function expressions break to multiple lines when repeated.

```ts:main.ts
call(function () { return foo; }, function () { return bar; })
```

```ts expected
call(
    function () {
        return foo;
    },
    function () {
        return bar;
    },
);
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

When chains exceed line width, each method gets its own line with semicolon on last line.

```ds line-width=30
data.filter(x => x.active).map(x => x.name).join(", ")
```

```ds expected
data.filter((x) => x.active)
    .map((x) => x.name)
    .join(", ");
```

### promise chain

Promise chains break nicely across lines with semicolon on last line.

```ds line-width=40
fetch(url).then(r => r.json()).then(data => process(data)).catch(handleError)
```

```ds expected
fetch(url)
    .then((r) => r.json())
    .then((data) => process(data))
    .catch(handleError);
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

### long binary expression breaks

Long binary expressions break at operators with all operands at same indentation.

```ds line-width=30
result = aLongVariableName + anotherLongName + thirdLongName
```

```ds expected
result =
    aLongVariableName +
    anotherLongName +
    thirdLongName;
```

### long binary declarator breaks after equals

Long binary declarators also break after `=` when needed.

```ds line-width=40
const sum = aLongVariableName + anotherLongName + thirdLongName
```

```ds expected
const sum =
    aLongVariableName +
    anotherLongName +
    thirdLongName;
```

### binary with logical operators

Logical operators break the same way.

```ds line-width=40
const isValid = hasPermission && isActive && !isDisabled
```

```ds expected
const isValid =
    hasPermission &&
    isActive &&
    !isDisabled;
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
