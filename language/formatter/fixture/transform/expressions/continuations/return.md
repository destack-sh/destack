# Return Statements

## Return

### return void

Return without a value exits the function.

```tspp
return
```

```tspp expected
return;
```

### return value

Return with a value produces that value from the function.

```tspp
return value
```

```tspp expected
return value;
```

### return expression

Expressions can be returned directly.

```tspp
return a + b
```

```tspp expected
return a + b;
```

### return object

Object literals can be returned directly.

```tspp
return { x: 1, y: 2 }
```

```tspp expected
return { x: 1, y: 2 };
```
