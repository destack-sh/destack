# Wildcard Patterns

## discard

### wildcard binding discards a value

`_` can be used where a value must be matched but not named.

```ds
let _ = 1;
```

### wildcard does not introduce a binding

Values matched by `_` cannot be read later.

```ds
let _ = 1;
_ satisfies number;
```

- contains: missing symbol

### wildcard discards tuple positions

Wildcards can discard individual tuple positions.

```ds
let (_, value) = (1, "ok");
value satisfies string;
```

### wildcard discards object fields

Wildcards can discard selected object fields.

```ds
let { id: _, name } = { id: 1, name: "Ada" };
name satisfies string;
```
