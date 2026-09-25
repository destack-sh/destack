# Break and Continue

## Break and Continue

### break statement

Break statements exit the innermost loop.

```tspp
break
```

```tspp expected
break;
```

### labeled break

Break labels preserve their names.

```tspp
break outer
```

```tspp expected
break outer;
```

### break value

Bare identifier break values use parentheses to avoid label ambiguity.

```tspp
break (value)
```

```tspp expected
break (value);
```

### labeled break value

Labeled break values use a colon before the value.

```tspp
break outer: value
```

```tspp expected
break outer: value;
```

### continue statement

Continue statements skip to the next iteration.

```tspp
continue
```

```tspp expected
continue;
```

### labeled continue

Continue labels preserve their names.

```tspp
continue outer
```

```tspp expected
continue outer;
```
