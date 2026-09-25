# Fixed Arrays

## Fixed Arrays

### fixed array repeat literal

Fixed array repeat literals keep Rust-style semicolon syntax.

```tspp
const zeros: [uint8; 32] = [0; 32]
```

```tspp expected
const zeros: [uint8; 32] = [0; 32];
```

### fixed array repeat literal with expressions

Repeat values and lengths can be computed expressions.

```tspp
const values = [factory(index + 1); width * height]
```

```tspp expected
const values = [factory(index + 1); width * height];
```

### fixed array repeat literal comments

Comments around the repeated value and length stay inside the repeat literal.

```tspp
const values = [seed /* value */; /* length */ count]
```

```tspp expected
const values = [seed /* value */; /* length */ count];
```

### fixed array repeat literal line comment

A line comment after the separator breaks the repeat length onto the next line.

```tspp
const values = [seed; // length
count]
```

```tspp expected
const values = [
    seed; // length
    count
];
```
