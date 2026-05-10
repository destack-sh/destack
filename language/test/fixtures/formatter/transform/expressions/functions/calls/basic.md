# Function Calls

## Call Forms

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

## Optional Chaining

### optional method call

Optional chaining uses `?.` for nullable access.

```ds
obj?.method()
```

```ds expected
obj?.method();
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
