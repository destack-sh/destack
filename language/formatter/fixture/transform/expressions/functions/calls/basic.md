# Function Calls

## Call Forms

### function calls have no internal spacing

Spaces after `(` and before `)` should be removed.

```tspp
foo( a, b, c )
```

```tspp expected
foo(a, b, c);
```

### short argument lists stay on one line

Short calls remain on a single line.

```tspp
foo(a, b, c)
```

```tspp expected
foo(a, b, c);
```

### empty function call

Empty argument lists have all spaces removed.

```tspp
foo(   )
```

```tspp expected
foo();
```

### blank lines between callee and arguments

Blank lines between the callee and argument list are removed.

```tspp:main.tspp
gen

    ("a");

gen

    (

      "b");
```

```tspp expected
gen("a");

gen("b");
```

### single argument call

Single arguments have internal spacing removed.

```tspp
foo(   x   )
```

```tspp expected
foo(x);
```

### call on instantiation expression

Calls on parenthesized instantiation expressions keep the parentheses.

```tspp:main.tspp
const value = (makeFactory<number>)(config)
```

```tspp expected
const value = makeFactory<number>(config);
```

## Optional Chaining

### optional method call

Optional chaining uses `?.` for nullable access.

```tspp
obj?.method()
```

```tspp expected
obj?.method();
```

## Spread Arguments

### spread in function call

Spread operator expands arrays into arguments.

```tspp
foo(...args)
```

```tspp expected
foo(...args);
```

### mixed spread and regular arguments

Spread can appear anywhere in the argument list.

```tspp
foo(a, b, ...rest, c)
```

```tspp expected
foo(a, b, ...rest, c);
```
