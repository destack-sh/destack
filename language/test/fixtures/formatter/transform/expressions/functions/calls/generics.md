# Generic Calls

## Generic Calls

### function call with type arguments

Generic type arguments appear in angle brackets.

```ds
foo<number>(x)
```

```ds expected
foo<number>(x);
```

### call with multiline commented type arguments

Comments inside multiline call type arguments keep the type arguments multiline.

```ts:main.ts
Math.random<
  // comment
  string | number | undefined
>()
```

```ts expected
Math.random<
    // comment
    string | number | undefined
>();
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

Method calls can also have type arguments.
Arrow function params get parentheses.

```ds
array.map<string>(x => x.toString())
```

```ds expected
array.map<string>((x) => x.toString());
```
