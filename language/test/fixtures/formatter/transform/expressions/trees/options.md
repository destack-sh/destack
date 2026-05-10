# Tree Formatting Options

## JSX Formatting Options

### single attribute per line forces expansion

When `single_attribute_per_line` is true, multiple attributes each get their own line.

```ds single-attribute-per-line=true
<Button variant="primary" size="large" />
```

```ds expected
<Button
    variant="primary"
    size="large"
/>;
```

### bracket same line keeps self-closing slash on its own line

For self-closing tags, `bracket_same_line` does not pull `/>` up onto the last attribute line.

```ds bracket-same-line=true line-width=30
<Button variant="primary" size="large" disabled />
```

```ds expected
<Button
    variant="primary"
    size="large"
    disabled
/>;
```

### single attr per line with bracket same line combined

Both options can be combined while still keeping the self-closing `/>` on its own line.

```ds single-attribute-per-line=true bracket-same-line=true
<Button variant="primary" size="large" onClick={handleClick} />
```

```ds expected
<Button
    variant="primary"
    size="large"
    onClick={handleClick}
/>;
```
