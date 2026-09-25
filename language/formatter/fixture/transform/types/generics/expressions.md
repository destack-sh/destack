# Generics

Generic fixtures cover type arguments on calls, members, and instantiation expressions.

## Type Arguments

### generic brackets have no internal spacing

Spaces inside angle brackets should be removed.

```tspp
const x: Array< number > = []
```

```tspp expected
const x: Array<number> = [];
```

### generic with multiple parameters

Multiple type parameters are separated by comma and space.

```tspp
const x: Map< string , number > = new Map()
```

```tspp expected
const x: Map<string, number> = new Map();
```

### generic with comments

Comments inside type arguments are preserved.

```tspp
const x: Map</* key */ string, /* value */ number> = new Map()
```

```tspp expected
const x: Map</* key */ string, /* value */ number> = new Map();
```

### spread type argument

Spread type arguments keep the spread marker attached to the argument.

```tspp
const tensor: Tensor< ...Shape > = value
```

```tspp expected
const tensor: Tensor<...Shape> = value;
```

### spread value argument

Spread value arguments keep expression spacing inside the argument.

```tspp
const buffer: Buffer< ...shape() > = value
```

```tspp expected
const buffer: Buffer<...shape()> = value;
```

## Instantiation Expressions

### instantiation keeps type arguments inline

Instantiation expressions should retain their type arguments without extra spacing.

```tspp:main.tspp
const factory = getFactory<number>
```

```tspp expected
const factory = getFactory<number>;
```

### instantiation with multiple parameters

Multiple type arguments are separated by comma and space.

```tspp:main.tspp
const pair = makePair<string, number>
```

```tspp expected
const pair = makePair<string, number>;
```

### instantiation with comments

Comments inside instantiation type arguments are preserved.

```tspp:main.tspp
const pair = makePair</* key */ string, /* value */ number>
```

```tspp expected
const pair = makePair</* key */ string, /* value */ number>;
```

### instantiation with multiline comments

Comments inside multiline instantiation type arguments keep the type arguments multiline.

```tspp:main.tspp
Math.random<
  // comment
  string | number | undefined
>
```

```tspp expected
Math.random<
    // comment
    string | number | undefined
>;
```
