# Await Using Declarations

## Await Using Declarations

### await using declaration

Await using preserves the `await` keyword.

```tspp
await using conn = open()
```

```tspp expected
await using conn = open();
```

### await using with call arguments

Await using declarations preserve initializer call arguments.

```tspp
await using resource = openAsync(path)
```

```tspp expected
await using resource = openAsync(path);
```
