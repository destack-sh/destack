# Fixed Arrays

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
