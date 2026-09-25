# Generic Calls

## Generic Calls

### function call with type arguments

Generic type arguments appear in angle brackets.

```tspp
foo<number>(x)
```

```tspp expected
foo<number>(x);
```

### call with multiline commented type arguments

Comments inside multiline call type arguments keep the type arguments multiline.

```tspp:main.tspp
Math.random<
  // comment
  string | number | undefined
>()
```

```tspp expected
Math.random<
    // comment
    string | number | undefined
>();
```

### function call with multiple type arguments

Multiple type arguments are separated by comma and space.

```tspp
foo<number, string, boolean>(x, y, z)
```

```tspp expected
foo<number, string, boolean>(x, y, z);
```

### method call with type arguments

Method calls can also have type arguments.
Arrow function params get parentheses.

```tspp
array.map<string>(x => x.toString())
```

```tspp expected
array.map<string>((x) => x.toString());
```
