# Newtype Aliases

## Newtype Aliases

### newtype scalar

Newtype aliases format like type aliases with the `newtype` keyword.

```ds
newtype UserId = int64
```

```ds expected
newtype UserId = int64;
```

### newtype tuple

Newtype tuples keep tuple formatting.

```ds
newtype Point = (float32, float32)
```

```ds expected
newtype Point = (float32, float32);
```

### singleton newtype tuple

Singleton newtype tuples keep the required tuple comma.

```ds
newtype extern = (string,)
```

```ds expected
newtype extern = (string,);
```

### spread newtype tuple

Parenthesized tuple rest elements keep array suffixes on the rest type.

```ds
newtype require = (...HostAction[])
```

```ds expected
newtype require = (...HostAction[],);
```

### newtype object

Newtype object values keep object literal formatting.

```ds
newtype Config = { debug: boolean }
```

```ds expected
newtype Config = { debug: boolean };
```
