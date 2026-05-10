# Array Literals

Array fixtures cover array spacing, comments, spread elements, line breaking, and nested values.

## Spacing

### array brackets have no internal spacing

Spaces after `[` and before `]` should be removed.

```ds
[ 1, 2, 3 ]
```

```ds expected
[1, 2, 3];
```

### short arrays stay on one line

Short array literals remain on a single line.

```ds
[1, 2, 3, 4, 5]
```

```ds expected
[1, 2, 3, 4, 5];
```

### empty array

Empty arrays have all spaces removed.

```ds
[   ]
```

```ds expected
[];
```

### single element array

Single elements have internal spacing removed.

```ds
[   1   ]
```

```ds expected
[1];
```

### array normalizes comma spacing

Commas get a space after but not before.

```ds
[1,2,3]
```

```ds expected
[1, 2, 3];
```

### array removes extra spaces

Extra spaces around elements are normalized.

```ds
[  1  ,  2  ,  3  ]
```

```ds expected
[1, 2, 3];
```

## Comments

### array element sibling comments

Trailing element comments and next element leading comments keep separate ownership.

```ds
const value = [first, // first
// second
second]
```

```ds expected
const value = [
    first, // first
    // second
    second,
];
```

## Line Breaking

### array breaks when line width is exceeded

When the line width is exceeded, arrays break to multiple lines.

```ds line-width=10
[1, 2, 3, 4, 5]
```

```ds expected
[
    1, 2,
    3, 4,
    5,
];
```

### array with long elements breaks

Long string elements also trigger line breaking.

```ds line-width=30
["longString", "anotherLong", "third"]
```

```ds expected
[
    "longString",
    "anotherLong",
    "third",
];
```

### array of identifiers breaks

Identifier arrays also break when exceeding line width.

```ds line-width=30
[firstName, lastName, email, phone]
```

```ds expected
[
    firstName,
    lastName,
    email,
    phone,
];
```

### array preserves single line when fits

Arrays stay on one line when they fit within line width.

```ds line-width=50
[1, 2, 3, 4, 5]
```

```ds expected
[1, 2, 3, 4, 5];
```

## Spread Elements

### array with spread

Spread elements stay tight with the ellipsis.

```ds
[head, ...rest]
```

```ds expected
[head, ...rest];
```

### array with multiple spreads

Multiple spread elements keep spacing normalized.

```ds
[...left, middle, ...right]
```

```ds expected
[...left, middle, ...right];
```

## Nested Arrays

### nested array

Nested arrays break with one nested element per line.

```ds
[[1, 2], [3, 4]]
```

```ds expected
[
    [1, 2],
    [3, 4],
];
```

### deeply nested array

Any depth of nesting is preserved.

```ds
[[[1]]]
```

```ds expected
[[[1]]];
```

### nested array breaks at line width

Outer array breaks while inner arrays stay compact.

```ds line-width=20
[[1, 2, 3], [4, 5, 6], [7, 8, 9]]
```

```ds expected
[
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
];
```

### nested rows

Nested arrays format with one row per line.

```ds line-width=30
[[1, 0, 0], [0, 1, 0], [0, 0, 1]]
```

```ds expected
[
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
];
```

### array of objects breaks

Object elements break to one per line when they exceed width.

```ds line-width=30
[{ a: 1, b: 2 }, { c: 3, d: 4 }]
```

```ds expected
[
    { a: 1, b: 2 },
    { c: 3, d: 4 },
];
```

## Array of Objects

### array of objects stays on one line if short

Short object elements stay inline.

```ds
[{ a: 1 }, { a: 2 }]
```

```ds expected
[{ a: 1 }, { a: 2 }];
```

### array of complex objects

Complex objects break to separate lines.

```ds line-width=40
[{ id: 1, name: "first" }, { id: 2, name: "second" }]
```

```ds expected
[
    { id: 1, name: "first" },
    { id: 2, name: "second" },
];
```

## Spread

### spread in array

Spread operator expands iterables inline.

```ds
[...items]
```

```ds expected
[...items];
```

### spread with elements

Spread can be mixed with regular elements.

```ds
[1, ...items, 2]
```

```ds expected
[1, ...items, 2];
```

### multiple spreads

Multiple spreads can appear in one array.

```ds
[...a, ...b, ...c]
```

```ds expected
[...a, ...b, ...c];
```

### spread at start

Spread can appear at the beginning.

```ds
[...prefix, 1, 2, 3]
```

```ds expected
[...prefix, 1, 2, 3];
```

### spread at end

Spread can appear at the end.

```ds
[1, 2, 3, ...suffix]
```

```ds expected
[1, 2, 3, ...suffix];
```

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

## Trailing Comma

### short array without trailing comma

Short arrays on one line don't get trailing commas.

```ds
[1, 2, 3]
```

```ds expected
[1, 2, 3];
```

## Type Annotations

### typed array

Array type annotations use `T[]` syntax.

```ds
const x: number[] = [1, 2, 3]
```

```ds expected
const x: number[] = [1, 2, 3];
```

### array with as

`as const` makes array literal readonly.

```ds
[1, 2, 3] as const
```

```ds expected
[1, 2, 3] as const;
```

### array satisfies type

`satisfies` checks type without changing it.

```ds
[1, 2, 3] satisfies number[]
```

```ds expected
[1, 2, 3] satisfies number[];
```

## Complex Arrays

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

## Single Element Expansion

### single object in array expands outer array

When an array contains a single object that expands, the outer array also expands.

```ds line-width=20
[{ a: 1, b: 2, c: 3 }]
```

```ds expected
[
    {
        a: 1,
        b: 2,
        c: 3,
    },
];
```

### single array in array expands outer array

Nested arrays also expand when the single nested array element expands.

```ds line-width=20
[[1, 2, 3, 4, 5, 6]]
```

```ds expected
[
    [
        1, 2, 3, 4,
        5, 6,
    ],
];
```

## Fixed Arrays

### fixed array repeat literal

Fixed array repeat literals keep Rust-style semicolon syntax.

```ds
const zeros: [uint8; 32] = [0; 32]
```

```ds expected
const zeros: [uint8; 32] = [0; 32];
```

### fixed array repeat literal with expressions

Repeat values and lengths can be computed expressions.

```ds
const values = [factory(index + 1); width * height]
```

```ds expected
const values = [factory(index + 1); width * height];
```

### fixed array repeat literal comments

Comments around the repeated value and length stay inside the repeat literal.

```ds
const values = [seed /* value */; /* length */ count]
```

```ds expected
const values = [seed /* value */; /* length */ count];
```

### fixed array repeat literal line comment

A line comment after the separator breaks the repeat length onto the next line.

```ds
const values = [seed; // length
count]
```

```ds expected
const values = [
    seed; // length
    count
];
```

## Type Assertions

### array as const

Const assertions stay on the same line as the array literal.

```ts:main.ts
const values = [1, 2, 3] as const
```

```ts expected
const values = [1, 2, 3] as const;
```
