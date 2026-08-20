# Using Declarations

## Using Declarations

### using declaration

Using declarations keep spacing around `=`.

```ds
using resource=open()
```

```ds expected
using resource = open();
```

### using with call arguments

Using declarations preserve initializer call arguments.

```ds
using resource = open(path)
```

```ds expected
using resource = open(path);
```

