# Using Declarations

## Using Declarations

### using declaration

Using declarations keep spacing around `=`.

```tspp
using resource=open()
```

```tspp expected
using resource = open();
```

### using with call arguments

Using declarations preserve initializer call arguments.

```tspp
using resource = open(path)
```

```tspp expected
using resource = open(path);
```

