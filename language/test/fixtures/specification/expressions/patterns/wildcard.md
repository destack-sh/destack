# Wildcard Patterns

## discard

### wildcard binding discards a value

> `_` can be used where a value must be matched but not named.

```ds
let _ = 1;
```

### wildcard does not introduce a binding

> Values matched by `_` cannot be read later.

```ds
let _ = 1;
_ satisfies int32;
```

- contains: missing symbol

### wildcard can appear inside tuple patterns

> Wildcards can discard individual tuple positions.

```ds
let (_, value) = (1, "ok");
value satisfies string;
```

### wildcard can appear inside object patterns

> Wildcards can discard selected object fields.

```ds
let { id: _, name } = { id: 1, name: "Ada" };
name satisfies string;
```
