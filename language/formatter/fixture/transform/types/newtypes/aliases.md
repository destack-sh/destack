# Newtype Aliases

## Newtype Aliases

### newtype scalar

Newtype aliases format like type aliases with the `newtype` keyword.

```tspp
newtype UserId = int64
```

```tspp expected
newtype UserId = int64;
```

### newtype tuple

Newtype tuples keep tuple formatting.

```tspp
newtype Point = (float32, float32)
```

```tspp expected
newtype Point = (float32, float32);
```

### singleton newtype tuple

Singleton newtype tuples keep the required tuple comma.

```tspp
newtype extern = (string,)
```

```tspp expected
newtype extern = (string,);
```

### spread newtype tuple

Parenthesized tuple rest elements keep array suffixes on the rest type.

```tspp
newtype require = (...HostAction[])
```

```tspp expected
newtype require = (...HostAction[],);
```

### newtype object

Newtype object values keep object literal formatting.

```tspp
newtype Config = { debug: boolean }
```

```tspp expected
newtype Config = { debug: boolean };
```
