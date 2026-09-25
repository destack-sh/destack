# Array Expressions

## Array with Expressions

### array with function calls

Function calls as elements are preserved.

```tspp
[foo(), bar(), baz()]
```

```tspp expected
[foo(), bar(), baz()];
```

### array with binary expressions

Expressions as elements are preserved.

```tspp
[a + b, c * d, e - f]
```

```tspp expected
[a + b, c * d, e - f];
```

### array with ternary

Ternary expressions as elements are preserved.

```tspp
[x ? 1 : 2, y ? 3 : 4]
```

```tspp expected
[x ? 1 : 2, y ? 3 : 4];
```

### array with arrow functions

Arrow functions get parentheses added to params.

```tspp
[x => x, y => y * 2]
```

```tspp expected
[(x) => x, (y) => y * 2];
```

### array with template literals

Template literals are preserved as-is.

```tspp
[`a`, `b`, `c`]
```

```tspp expected
[`a`, `b`, `c`];
```

## Array Positions

### array as function argument

Arrays can be passed directly as arguments.

```tspp
foo([1, 2, 3])
```

```tspp expected
foo([1, 2, 3]);
```

### array in object property

Arrays can be object property values.

```tspp
const x = { items: [1, 2, 3] }
```

```tspp expected
const x = { items: [1, 2, 3] };
```

### array destructuring target

Array destructuring extracts elements by position.

```tspp
const [a, b, c] = [1, 2, 3]
```

```tspp expected
const [a, b, c] = [1, 2, 3];
```

### array map chain

Method chains on array literals stay attached.
Arrow params get parens.

```tspp
[1, 2, 3].map(x => x * 2)
```

```tspp expected
[1, 2, 3].map((x) => x * 2);
```
