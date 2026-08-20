# Return Statements

## Return

### return void

Return without a value exits the function.

```ds
return
```

```ds expected
return;
```

### return value

Return with a value produces that value from the function.

```ds
return value
```

```ds expected
return value;
```

### return expression

Expressions can be returned directly.

```ds
return a + b
```

```ds expected
return a + b;
```

### return object

Object literals can be returned directly.

```ds
return { x: 1, y: 2 }
```

```ds expected
return { x: 1, y: 2 };
```
