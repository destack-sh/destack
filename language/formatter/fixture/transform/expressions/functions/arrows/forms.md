# Arrow Forms

## Arrow Function Heads

### parenthesized arrow parameter

Arrow functions with parenthesized params are preserved.

```ds
const f = (x) => x
```

```ds expected
const f = (x) => x;
```

### arrow function without parens for single param

Destack always adds parentheses around arrow function parameters.

```ds
const f = x => x
```

```ds expected
const f = (x) => x;
```

### arrow function with no parameters

Zero-parameter arrow functions use empty parentheses.

```ds
const f = () => 1
```

```ds expected
const f = () => 1;
```

### arrow function with multiple parameters

Multiple parameters are comma-separated.

```ds
const f = (a, b, c) => a + b + c
```

```ds expected
const f = (a, b, c) => a + b + c;
```

### arrow function normalizes spacing

Extra spacing is normalized to single spaces.

```ds
const f = (  a  ,  b  )  =>  a + b
```

```ds expected
const f = (a, b) => a + b;
```

## Immediately Invoked

### iife arrow function

IIFEs wrap and immediately invoke the arrow function.

```ds
(() => { console.log("hello") })()
```

```ds expected
(() => {
    console.log("hello")
})();
```

### iife with argument

IIFEs can pass arguments to the invoked function.

```ds
((x) => x * 2)(5)
```

```ds expected
((x) => x * 2)(5);
```

## Returning Objects

### arrow function returning object needs parens

Object returns need parentheses to avoid brace ambiguity.

```ds
const f = () => ({ x: 1, y: 2 })
```

```ds expected
const f = () => ({ x: 1, y: 2 });
```

### arrow function returning complex object

Complex objects with shorthand properties work.

```ds
const f = (name) => ({ name, value: 1, active: true })
```

```ds expected
const f = (name) => ({ name, value: 1, active: true });
```

