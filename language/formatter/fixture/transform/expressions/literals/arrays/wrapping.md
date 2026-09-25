# Array Wrapping

## Line Breaking

### array breaks when line width is exceeded

When the line width is exceeded, arrays break to multiple lines.

```tspp line-width=10
[1, 2, 3, 4, 5]
```

```tspp expected
[
    1, 2,
    3, 4,
    5,
];
```

### array with long elements breaks

Long string elements also trigger line breaking.

```tspp line-width=30
["longString", "anotherLong", "third"]
```

```tspp expected
[
    "longString",
    "anotherLong",
    "third",
];
```

### array of identifiers breaks

Identifier arrays also break when exceeding line width.

```tspp line-width=30
[firstName, lastName, email, phone]
```

```tspp expected
[
    firstName,
    lastName,
    email,
    phone,
];
```

### array preserves single line when fits

Arrays stay on one line when they fit within line width.

```tspp line-width=50
[1, 2, 3, 4, 5]
```

```tspp expected
[1, 2, 3, 4, 5];
```
