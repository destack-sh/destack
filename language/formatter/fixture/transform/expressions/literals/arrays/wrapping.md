# Array Wrapping

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
