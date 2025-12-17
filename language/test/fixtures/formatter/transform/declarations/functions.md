# Function Declarations

Tests for function declaration formatting.

## Basic Functions

### simple function

Extra whitespace in the function signature should be removed.

```ds
function   foo  (  )   {   }
```

Empty function bodies stay on one line with a space inside the braces.

```ds expected
function foo() { }
```

### function with parameters

Parameter spacing should be normalized with no space after `(` or before `)`.

```ds
function   foo  (  x  :  number  ,  y  :  string  )   {   }
```

```ds expected
function foo(x: number, y: string) { }
```

### function with return type

Functions with statements in the body get their body broken to multiple lines.

```ds
function   foo  (  )  :  number   {  return 1  }
```

```ds expected
function foo(): number {
    return 1
}
```

## Async Functions

### async function

```ds
async   function   foo  (  )   {   }
```

```ds expected
async function foo() { }
```

## Generator Functions

### generator function

The `*` attaches to the `function` keyword with no space.

```ds
function  *  foo  (  )   {   }
```

```ds expected
function* foo() { }
```

## Arrow Functions

### arrow function expression

Arrow functions with expression bodies stay on one line.

```ds
const   foo   =   (  x  )   =>   x  +  1
```

```ds expected
const foo = (x) => x + 1;
```

### arrow function with body

Arrow functions with block bodies get broken to multiple lines.

```ds
const   foo   =   (  x  )   =>   {   return  x  +  1   }
```

```ds expected
const foo = (x) => {
    return x + 1
};
```
