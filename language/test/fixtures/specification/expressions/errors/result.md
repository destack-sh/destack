# Result Types

`Result<T, E>` assignability and narrowing.

## constructors

### result constructors return Result

> Result constructors produce the declared result type.

```ds
const ok: Result<int, string> = Result.ok(1);
const err: Result<int, string> = Result.err("no");

ok satisfies Result<int, string>;
err satisfies Result<int, string>;
```

### ok and err values are not Result without wrapping

> Result is nominal and requires explicit construction.

```ds
const value: Result<int, string> = Ok { value: 1 };
```

- contains: not assignable

## narrowing

### discriminant narrows result variants

> Discriminant checks narrow to `Ok` and `Err` variants.

```ds
const value: Result<int, string> = Result.ok(1);

if (value.kind == "Ok") {
    value.value satisfies int;
} else {
    value.error satisfies string;
}
```
