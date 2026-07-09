# Quoted Object Properties

## Quoted Properties

### quoted property names

Property names that require quotes stay quoted.

```ds
const x = { "data-id": 1, "default": 2 }
```

```ds expected
const x = { "data-id": 1, default: 2 };
```

### mixed quoted and unquoted

Only properties that require quotes stay quoted.

```ds
const x = { normal: 1, "needs-quotes": 2 }
```

```ds expected
const x = { normal: 1, "needs-quotes": 2 };
```

### quote properties as needed

Quotes are removed when they are not required.

```ds:main.ds
const x = { "data-id": 1, "default": 2, "normal": 3 }
```

```ds expected
const x = { "data-id": 1, default: 2, normal: 3 };
```

## Unicode Keys

### object with unicode keys

Unicode keys that are not identifiers stay quoted and normalize quotes.

```ds:main.ds
x = { 'x・': 0, 'x･': 1 }
```

```ds expected
x = { "x・": 0, "x･": 1 };
```
