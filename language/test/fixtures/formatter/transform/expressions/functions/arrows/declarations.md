# Arrow Function Declarations

## Arrow Functions

### arrow function expression

Arrow functions with expression bodies stay on one line.
Single parameters get parentheses.

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
    return x + 1;
};
```

### arrow function with type

Type annotations on arrow function variables are preserved.

```ds
const foo: (x: number) => number = (x) => x + 1
```

```ds expected
const foo: (x: number) => number = (x) => x + 1;
```

### module ts arrow generic keeps trailing comma in mts

Single generic arrow type parameters in `.mts` keep a trailing comma.

```ts:main.mts
const fn = <T,>() => {}
```

```ts expected
const fn = <T,>() => {};
```

### module ts arrow generic keeps trailing comma in cts

Single generic arrow type parameters in `.cts` keep a trailing comma.

```ts:main.cts
const fn = <T,>() => {}
```

```ts expected
const fn = <T,>() => {};
```
