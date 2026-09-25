# Quoted Object Properties

## Quoted Properties

### quoted property names

Property names that require quotes stay quoted.

```tspp
const x = { "data-id": 1, "default": 2 }
```

```tspp expected
const x = { "data-id": 1, default: 2 };
```

### mixed quoted and unquoted

Only properties that require quotes stay quoted.

```tspp
const x = { normal: 1, "needs-quotes": 2 }
```

```tspp expected
const x = { normal: 1, "needs-quotes": 2 };
```

### quote properties as needed

Quotes are removed when they are not required.

```tspp:main.tspp
const x = { "data-id": 1, "default": 2, "normal": 3 }
```

```tspp expected
const x = { "data-id": 1, default: 2, normal: 3 };
```

## Unicode Keys

### object with unicode keys

Unicode keys that are not identifiers stay quoted and normalize quotes.

```tspp:main.tspp
x = { 'x・': 0, 'x･': 1 }
```

```tspp expected
x = { "x・": 0, "x･": 1 };
```
