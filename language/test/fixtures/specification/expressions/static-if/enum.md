# Static If Enums

Enum fields can be gated with `@if`.

## gating

### when false, `@if` removes enum fields

When false, `@if` removes enum fields.

```ds
enum Status {
    @if(false)
    Missing = missingSymbol,
    Visible = 1,
}
```

### when true, `@if` includes enum fields

When true, `@if` includes enum fields.

```ds
enum Status {
    @if(true)
    Active = 1,
    Inactive = 2,
}

const active = Status.Active;
active satisfies Status;
```

### when false, `@if` hides enum fields

When false, enum fields behind `@if` are not available by name.

```ds
enum Status {
    @if(false)
    Missing = 0,
    Visible = 1,
}

const value = Status.Missing;
value satisfies Status;
```

- contains: does not exist

### when false, `@if` leaves other enum fields alone

Disabled enum fields do not affect lookup of enabled fields.

```ds
enum Status {
    @if(false)
    Missing = 0,
    Ready = 1,
    Done = 2,
}

const ready = Status.Ready;
ready satisfies Status;
```

### when true, `@if` keeps enum fields available

When true, enum fields behind `@if` remain available by name.

```ds
enum Status {
    @if(true)
    Ready = 1,
    Done = 2,
}

const ready = Status.Ready;
ready satisfies Status;
```
