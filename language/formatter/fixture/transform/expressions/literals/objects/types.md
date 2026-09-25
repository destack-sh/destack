# Object Type Annotations

## Type Annotations

### object with type annotation

Object type annotations use the same brace syntax.

```tspp
const x: { a: number } = { a: 1 }
```

```tspp expected
const x: { a: number } = { a: 1 };
```

### object satisfies type

`satisfies` checks type without changing inference.

```tspp
const x = { a: 1 } satisfies Record<string, number>
```

```tspp expected
const x = { a: 1 } satisfies Record<string, number>;
```

## As Const

### object as const

`as const` keeps the object literal inline when it fits.

```tspp:main.tspp
const settings = { retries: 3, verbose: false } as const
```

```tspp expected
const settings = { retries: 3, verbose: false } as const;
```
