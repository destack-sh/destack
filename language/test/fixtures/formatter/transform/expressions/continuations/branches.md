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

Break labels use TypeScript label names.

```ds
break outer
```

```ds expected
break outer;
```

### break value

Unlabeled break values use parentheses.

```ds
break (value)
```

```ds expected
break (value);
```

### labeled break value

Labeled break values use a colon before the value.

```ds
break outer: value
```

```ds expected
break outer: value;
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

Continue labels use TypeScript label names.

```ds
continue outer
```

```ds expected
continue outer;
```
