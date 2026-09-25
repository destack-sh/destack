# Regex Literals

## Regex Literals

### regex literal

Regex literals use forward slashes.

```tspp
const x = /pattern/
```

```tspp expected
const x = /pattern/;
```

### regex with flags

Flags follow the closing slash.

```tspp
const x = /\d+/g
```

```tspp expected
const x = /\d+/g;
```

### regex with multiple flags

Multiple flags can be combined.

```tspp
const x = /hello/gi
```

```tspp expected
const x = /hello/gi;
```

### regex with escaped pattern

Complex patterns are preserved exactly.

```tspp
const x = /^[a-z]+$/i
```

```tspp expected
const x = /^[a-z]+$/i;
```
