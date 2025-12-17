# Function Calls

Tests for function call expression formatting.

## Basic Calls

### function calls have no internal spacing

Spaces after `(` and before `)` should be removed.

```ds
foo( a, b, c )
```

```ds expected
foo(a, b, c);
```

### short argument lists stay on one line

Short calls remain on a single line.

```ds
foo(a, b, c)
```

```ds expected
foo(a, b, c);
```

## Method Chains

### short chains stay on one line

Short method chains remain on a single line.

```ds
foo().bar().baz()
```

```ds expected
foo().bar().baz();
```

## Binary Expressions

### short binary expressions stay on one line

```ds
a + b + c + d
```

```ds expected
a + b + c + d;
```
