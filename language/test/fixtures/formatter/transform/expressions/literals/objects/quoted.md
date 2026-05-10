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

### TypeScript quote props as needed

TypeScript removes quotes when they are not required.

```ts:main.ts
const x = { "data-id": 1, "default": 2, "normal": 3 }
```

```ts expected
const x = { "data-id": 1, default: 2, normal: 3 };
```

## Unicode Keys

### object with unicode keys

Unicode keys that are not identifiers stay quoted and normalize quotes.

```ts:main.ts
x = { 'x・': 0, 'x･': 1 }
```

```ts expected
x = { "x・": 0, "x･": 1 };
```
