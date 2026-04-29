# Using Statements

Using fixtures cover using and await-using declarations.

## Using Declarations

### using declaration

Using declarations keep spacing around `=`.

```ds
using resource=open()
```

```ds expected
using resource = open();
```

### await using declaration

Await using preserves the `await` keyword.

```ds
await using conn = open()
```

```ds expected
await using conn = open();
```

### using with call arguments

Using declarations preserve initializer call arguments.

```ds
using resource = open(path)
```

```ds expected
using resource = open(path);
```

### await using with call arguments

Await using declarations preserve initializer call arguments.

```ds
await using resource = openAsync(path)
```

```ds expected
await using resource = openAsync(path);
```
