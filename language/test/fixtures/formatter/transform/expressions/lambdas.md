# Lambda Expressions

Tests for arrow function and lambda expression formatting.

## Basic Arrow Functions

### simple arrow function with parens

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

## Typed Arrow Functions

### arrow function with parameter types

Type annotations follow parameter names with colon.

```ds
const f = (x: number) => x * 2
```

```ds expected
const f = (x: number) => x * 2;
```

### arrow function with return type

Return types appear after the parameter list.

```ds
const f = (x: number): number => x * 2
```

```ds expected
const f = (x: number): number => x * 2;
```

### arrow function with complex types

Union types and multiple typed parameters are supported.

```ts:main.ts
const f = (a: string, b: number): string | number => a || b
```

```ts expected
const f = (a: string, b: number): string | number => a || b;
```

## Arrow Functions with Block Bodies

### arrow function with block body

Block bodies expand to multiple lines with indented content.

```ds
const f = (x) => { return x * 2 }
```

```ds expected
const f = (x) => {
    return x * 2;
};
```

### arrow function with multiple statements

Multiple statements require a block body.

```ds
const f = (x) => { const y = x * 2; return y + 1 }
```

```ds expected
const f = (x) => {
    const y = x * 2;
    return y + 1;
};
```

### short arrow block with value tail

Short arrow blocks expand nested control-flow tails.

```ds
const f = (x: number): number => { const y = x * 2; if (y > 10) { y } else { y + 1 } }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    if (y > 10) {
        y
    } else {
        y + 1
    }
};
```

### expanded arrow block with value tail

Arrow blocks preserve semicolonless value tails.

```ds
const f = (x: number): number => { const y = x * 2; if (y > 10) { const capped = y - 1; capped } else { const boosted = y + 1; boosted } }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    if (y > 10) {
        const capped = y - 1;
        capped
    } else {
        const boosted = y + 1;
        boosted
    }
};
```

### arrow block with statement tail

Terminal semicolons in arrow blocks keep statement position.

```ds
const f = (x: number): number => { const y = x * 2; y; }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    y;
};
```

### empty block body

Empty blocks have internal spacing.

```ds
const f = () => { }
```

```ds expected
const f = () => {};
```

## Async Arrow Functions

### async arrow function

The `async` keyword precedes the parameter list.

```ds
const f = async (x) => x
```

```ds expected
const f = async (x) => x;
```

### async arrow function with await

Async functions can use `await` in their body.

```ds
const f = async (url) => await fetch(url)
```

```ds expected
const f = async (url) => await fetch(url);
```

### async arrow function with block

Async arrow functions with blocks expand normally.

```ds
const f = async (url) => { const res = await fetch(url); return res.json() }
```

```ds expected
const f = async (url) => {
    const res = await fetch(url);
    return res.json();
};
```

## Arrow Functions as Arguments

### callback in method call

Single param callbacks get parentheses added.

```ds
array.map(x => x * 2)
```

```ds expected
array.map((x) => x * 2);
```

### callback with parens

Parentheses are preserved when already present.

```ds
array.map((x) => x * 2)
```

```ds expected
array.map((x) => x * 2);
```

### callback with block body

Block bodies in callbacks expand to multiple lines.

```ds
array.forEach(item => { console.log(item) })
```

```ds expected
array.forEach((item) => {
    console.log(item)
});
```

### multiple callbacks

Each callback in a chain gets parentheses.

```ds
array.filter(x => x > 0).map(x => x * 2)
```

```ds expected
array.filter((x) => x > 0).map((x) => x * 2);
```

### callback with complex expression

Multi-param callbacks work with additional arguments.

```ds
array.reduce((acc, x) => acc + x, 0)
```

```ds expected
array.reduce((acc, x) => acc + x, 0);
```

## Destructuring Parameters

### arrow function with object destructuring

Object destructuring in params extracts named properties.

```ds
const f = ({ x, y }) => x + y
```

```ds expected
const f = ({ x, y }) => x + y;
```

### arrow function with array destructuring

Array destructuring extracts elements by position.

```ds
const f = ([a, b]) => a + b
```

```ds expected
const f = ([a, b]) => a + b;
```

### arrow function with renamed destructuring

Properties can be renamed during destructuring.

```ds
const f = ({ x: a, y: b }) => a + b
```

```ds expected
const f = ({ x: a, y: b }) => a + b;
```

### arrow function with default values

Default values use `=` with surrounding spaces.

```ds
const f = (x = 1) => x
```

```ds expected
const f = (x = 1) => x;
```

### arrow function with rest parameters

Rest parameters collect remaining arguments into an array.

```ds
const f = (...args) => args
```

```ds expected
const f = (...args) => args;
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

## Nested Arrow Functions

### nested arrow functions

Nested arrows each get parentheses added.

```ds
const f = x => y => x + y
```

```ds expected
const f = (x) => (y) => x + y;
```

### deeply nested arrow functions

Any depth of nesting gets consistent parentheses.

```ds
const f = a => b => c => a + b + c
```

```ds expected
const f = (a) => (b) => (c) => a + b + c;
```

### curried function with parens

Already-parenthesized curried functions are preserved.

```ds
const f = (a) => (b) => (c) => a + b + c
```

```ds expected
const f = (a) => (b) => (c) => a + b + c;
```

## Generic Arrow Functions

### arrow function with type parameter

```ds
const identity = <T,>(x: T): T => x
```

```ds expected
const identity = <T,>(x: T): T => x;
```

### arrow function with constrained type parameter

```ds
const first = <T: Iterable<U>, U>(items: T): U => items[0]
```

```ds expected
const first = <T: Iterable<U>, U>(items: T): U => items[0];
```

### arrow function with multiple type parameters

Multiple generic type parameters in function type syntax.

```ds
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn)
```

```ds expected
const map = <T, U>(arr: T[], fn: (x: T) => U): U[] => arr.map(fn);
```

## Line Breaking in Arrow Functions

### long arrow function breaks

Long arrow functions break at the assignment when needed.

```ds line-width=40
const processItem = (item) => transformAndValidate(item)
```

```ds expected
const processItem = (item) =>
    transformAndValidate(item);
```

### arrow function with long params breaks

Many parameters cause the param list to break.

```ds line-width=40
const fn = (first, second, third, fourth) => first + second
```

```ds expected
const fn = (
    first,
    second,
    third,
    fourth,
) => first + second;
```

### arrow function with complex return breaks

Complex return expressions break appropriately.

```ds line-width=50
const handler = (event) => ({ type: event.type, target: event.target, timestamp: Date.now() })
```

```ds expected
const handler = (event) => ({
    type: event.type,
    target: event.target,
    timestamp: Date.now(),
});
```

### arrow function with chained return breaks

Chain returns in arrow bodies keep each chain segment on its own line.

```ts:main.ts line-width=60
const normalize = (id) =>
  id
    .replace('@', resolve(__dirname, './mods/'))
    .replace('#', resolve(__dirname, '../../'))
```

```ts expected
const normalize = (id) =>
    id
        .replace("@", resolve(__dirname, "./mods/"))
        .replace("#", resolve(__dirname, "../../"));
```
