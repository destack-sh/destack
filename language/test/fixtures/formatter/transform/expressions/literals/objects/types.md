# Object Type Annotations

## Type Annotations

### object with type annotation

Object type annotations use the same brace syntax.

```ds
const x: { a: number } = { a: 1 }
```

```ds expected
const x: { a: number } = { a: 1 };
```

### object satisfies type

`satisfies` checks type without changing inference.

```ds
const x = { a: 1 } satisfies Record<string, number>
```

```ds expected
const x = { a: 1 } satisfies Record<string, number>;
```

## As Const

### object as const

`as const` keeps the object literal inline when it fits.

```ts:main.ts
const settings = { retries: 3, verbose: false } as const
```

```ts expected
const settings = { retries: 3, verbose: false } as const;
```
