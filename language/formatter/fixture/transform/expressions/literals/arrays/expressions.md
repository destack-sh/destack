# Array Expressions

## Array with Expressions

### array with function calls

Function calls as elements are preserved.

```ds
[foo(), bar(), baz()]
```

```ds expected
[foo(), bar(), baz()];
```

### array with binary expressions

Expressions as elements are preserved.

```ds
[a + b, c * d, e - f]
```

```ds expected
[a + b, c * d, e - f];
```

### array with ternary

Ternary expressions as elements are preserved.

```ds
[x ? 1 : 2, y ? 3 : 4]
```

```ds expected
[x ? 1 : 2, y ? 3 : 4];
```

### array with arrow functions

Arrow functions get parentheses added to params.

```ds
[x => x, y => y * 2]
```

```ds expected
[(x) => x, (y) => y * 2];
```

### array with template literals

Template literals are preserved as-is.

```ds
[`a`, `b`, `c`]
```

```ds expected
[`a`, `b`, `c`];
```

## Array Positions

### array as function argument

Arrays can be passed directly as arguments.

```ds
foo([1, 2, 3])
```

```ds expected
foo([1, 2, 3]);
```

### array in object property

Arrays can be object property values.

```ds
const x = { items: [1, 2, 3] }
```

```ds expected
const x = { items: [1, 2, 3] };
```

### array destructuring target

Array destructuring extracts elements by position.

```ds
const [a, b, c] = [1, 2, 3]
```

```ds expected
const [a, b, c] = [1, 2, 3];
```

### array map chain

Method chains on array literals stay attached.
Arrow params get parens.

```ds
[1, 2, 3].map(x => x * 2)
```

```ds expected
[1, 2, 3].map((x) => x * 2);
```
