# Tree Literals

## Self-Closing Elements

### self-closing tree element

Self-closing elements have a space before `/>`.

```tspp
<Entity  />
```

```tspp expected
<Entity />;
```

### self-closing with many attributes breaks

When attributes exceed line width, they break to multiple lines.

```tspp line-width=30
<Button variant="primary" size="large" disabled />
```

```tspp expected
<Button
    variant="primary"
    size="large"
    disabled
/>;
```
