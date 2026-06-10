# Unknown

`.ds` uses `unknown` for values that need explicit narrowing.

## unknown

### values assign to unknown

`unknown` is the top type.

```ds
const value: unknown = 42;
value satisfies unknown;
```

### unknown rejects arbitrary member access

Use requires narrowing first.

```ds
declare const value: unknown;
const result = value.missing;
```

- contains: unknown

## any

### any is rejected

`any` is forbidden in every source.

```ds
const value: any = 42;
```

- contains: any

### any does not enable dynamic access

There is no unchecked escape hatch.

```ds
declare const value: any;
const result = value.missing.member;
```

- contains: any
