# Arrow Forms

## Arrow Function Heads

### parenthesized arrow parameter

Arrow functions with parenthesized params are preserved.

```tspp
const f = (x) => x
```

```tspp expected
const f = (x) => x;
```

### arrow function without parens for single param

TS++ always adds parentheses around arrow function parameters.

```tspp
const f = x => x
```

```tspp expected
const f = (x) => x;
```

### arrow function with no parameters

Zero-parameter arrow functions use empty parentheses.

```tspp
const f = () => 1
```

```tspp expected
const f = () => 1;
```

### arrow function with multiple parameters

Multiple parameters are comma-separated.

```tspp
const f = (a, b, c) => a + b + c
```

```tspp expected
const f = (a, b, c) => a + b + c;
```

### arrow function normalizes spacing

Extra spacing is normalized to single spaces.

```tspp
const f = (  a  ,  b  )  =>  a + b
```

```tspp expected
const f = (a, b) => a + b;
```

## Immediately Invoked

### iife arrow function

IIFEs wrap and immediately invoke the arrow function.

```tspp
(() => { console.log("hello") })()
```

```tspp expected
(() => {
    console.log("hello")
})();
```

### iife with argument

IIFEs can pass arguments to the invoked function.

```tspp
((x) => x * 2)(5)
```

```tspp expected
((x) => x * 2)(5);
```

## Returning Objects

### arrow function returning object needs parens

Object returns need parentheses to avoid brace ambiguity.

```tspp
const f = () => ({ x: 1, y: 2 })
```

```tspp expected
const f = () => ({ x: 1, y: 2 });
```

### arrow function returning complex object

Complex objects with shorthand properties work.

```tspp
const f = (name) => ({ name, value: 1, active: true })
```

```tspp expected
const f = (name) => ({ name, value: 1, active: true });
```

