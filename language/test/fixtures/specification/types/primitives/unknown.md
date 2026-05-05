# Unknown

`.ds` uses `unknown` for values that need explicit narrowing.

## unknown

### values assign to unknown

```ds
const value: unknown = 42;
value satisfies unknown;
```

### unknown rejects arbitrary member access

```ds
declare const value: unknown;
const result = value.missing;
```

- contains: unknown

## any

### any is rejected

```ds
const value: any = 42;
```

- contains: any

### any does not enable dynamic access

```ds
declare const value: any;
const result = value.missing.member;
```

- contains: any
