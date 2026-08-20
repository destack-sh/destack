# Regex Literals

## Regex Literals

### regex literal

Regex literals use forward slashes.

```ds
const x = /pattern/
```

```ds expected
const x = /pattern/;
```

### regex with flags

Flags follow the closing slash.

```ds
const x = /\d+/g
```

```ds expected
const x = /\d+/g;
```

### regex with multiple flags

Multiple flags can be combined.

```ds
const x = /hello/gi
```

```ds expected
const x = /hello/gi;
```

### regex with escaped pattern

Complex patterns are preserved exactly.

```ds
const x = /^[a-z]+$/i
```

```ds expected
const x = /^[a-z]+$/i;
```
