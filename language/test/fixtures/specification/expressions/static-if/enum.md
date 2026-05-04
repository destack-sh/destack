# Static If Enums

Static if gating on enum fields.

## gating

### static if gates enum fields

> Enum fields gated by static if are removed before resolution.

```ds
enum Status {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    Missing = missingSymbol,
    Visible = 1,
}
```

### static if keeps enum fields when true

> Enum fields gated by true static if conditions remain available.

```ds
enum Status {
    @if(true)
    Active = 1,
    Inactive = 2,
}

const active = Status.Active;
active satisfies Status;
```

### static if gated enum fields are not visible

> Enum fields removed by static if are not available by name.

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

### static if gated enum members do not affect remaining members

> Disabled enum members do not affect lookup of enabled members.

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

### static if true enum members remain available

> Enum members gated with true remain available by name.

```ds
enum Status {
    @if(true)
    Ready = 1,
    Done = 2,
}

const ready = Status.Ready;
ready satisfies Status;
```
