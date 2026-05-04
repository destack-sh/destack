# Any

Portable `.ds` rejects `any`.
Use `unknown` and narrow or cast explicitly at interop boundaries.

## rejection

### explicit any is rejected in ds

> `any` is not a portable `.ds` type.

```ds
const value: any = 42;
```

- contains: any

### any does not enable member access in ds

> Dynamic member access through `any` is not available in portable `.ds`.

```ds
declare const value: any;
const result = value.missing.member;
```

- contains: any

## unknown

### unknown is the top type

> Values can be assigned to `unknown`.

```ds
const value: unknown = 42;
value satisfies unknown;
```

### unknown requires narrowing before use

> `unknown` does not allow arbitrary member access.

```ds
declare const value: unknown;
const result = value.missing;
```

- contains: unknown
