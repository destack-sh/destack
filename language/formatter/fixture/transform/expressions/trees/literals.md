# Tree Literals

## Self-Closing Elements

### self-closing tree element

Self-closing elements have a space before `/>`.

```ds
<Entity  />
```

```ds expected
<Entity />;
```

### self-closing with many attributes breaks

When attributes exceed line width, they break to multiple lines.

```ds line-width=30
<Button variant="primary" size="large" disabled />
```

```ds expected
<Button
    variant="primary"
    size="large"
    disabled
/>;
```
