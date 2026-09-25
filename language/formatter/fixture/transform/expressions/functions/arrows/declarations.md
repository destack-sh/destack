# Arrow Function Declarations

## Arrow Functions

### arrow function expression

Arrow functions with expression bodies stay on one line.
Single parameters get parentheses.

```tspp
const   foo   =   (  x  )   =>   x  +  1
```

```tspp expected
const foo = (x) => x + 1;
```

### arrow function with body

Arrow functions with block bodies get broken to multiple lines.

```tspp
const   foo   =   (  x  )   =>   {   return  x  +  1   }
```

```tspp expected
const foo = (x) => {
    return x + 1;
};
```

### arrow function with type

Type annotations on arrow function variables are preserved.

```tspp
const foo: (x: number) => number = (x) => x + 1
```

```tspp expected
const foo: (x: number) => number = (x) => x + 1;
```

### generic arrow keeps trailing comma

Single generic arrow type parameters keep the comma that disambiguates them from tree literals.

```tspp:main.tspp
const fn = <T,>() => {}
```

```tspp expected
const fn = <T,>() => {};
```
