# Tree Literals

Tests for Destack tree literal (JSX-like) formatting.

## Self-Closing Elements

### self-closing tree element

Self-closing elements have a space before `/>`.

```ds
<Entity  />
```

```ds expected
<Entity />;
```

## Elements with Attributes

### tree element with attributes

Attributes use `=` with no surrounding spaces.

```ds
<Entity  a = 1  b = 2  />
```

```ds expected
<Entity a=1 b=2 />;
```
