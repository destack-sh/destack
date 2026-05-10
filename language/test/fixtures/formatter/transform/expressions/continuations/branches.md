# Break and Continue

## Break and Continue

### break statement

Break statements exit the innermost loop.

```ds
break
```

```ds expected
break;
```

### labeled break

Destack uses colon prefix for labels: `break :label`.

```ds
break :outer
```

```ds expected
break :outer;
```

### continue statement

Continue statements skip to the next iteration.

```ds
continue
```

```ds expected
continue;
```

### labeled continue

Labeled continue uses the same colon prefix syntax.

```ds
continue :outer
```

```ds expected
continue :outer;
```
