# Await Using Declarations

## Await Using Declarations

### await using declaration

Await using preserves the `await` keyword.

```ds
await using conn = open()
```

```ds expected
await using conn = open();
```

### await using with call arguments

Await using declarations preserve initializer call arguments.

```ds
await using resource = openAsync(path)
```

```ds expected
await using resource = openAsync(path);
```
