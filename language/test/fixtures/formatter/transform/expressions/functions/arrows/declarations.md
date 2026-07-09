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

### generic arrow keeps trailing comma

Single generic arrow type parameters keep the comma that disambiguates them from tree literals.

```ds:main.ds
const fn = <T,>() => {}
```

```ds expected
const fn = <T,>() => {};
```
