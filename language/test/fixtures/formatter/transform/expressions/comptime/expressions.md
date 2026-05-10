# Comptime Expressions

## Comptime Forms

### comptime expression

Comptime expressions keep `comptime` tight to the body.

```ds
const value = comptime 1 + 2 + 3
```

```ds expected
const value = comptime 1 + 2 + 3;
```

### comptime operand values

Comptime values follow their inner expression shape in operand positions.

```ds
render(comptime buildView())
const value = (comptime createBuilder()).build()
```

```ds expected
render(comptime buildView());
const value = (comptime createBuilder()).build();
```

### comptime collection values

Comptime values keep spread and object-value grouping.

```ds
const values = [comptime buildValue(), ...(comptime buildValues())]
const envelope = { value: comptime buildValue() }
```

```ds expected
const values = [comptime buildValue(), ...comptime buildValues()];
const envelope = { value: comptime buildValue() };
```

### comptime parameter default value

Default parameters can use compact comptime values.

```ds
function render(view = comptime buildView()) { use(view) }
```

```ds expected
function render(view = comptime buildView()) {
    use(view)
}
```

### comptime template value

Compact comptime values stay inline inside template interpolations.

```ds
const label = `size: ${comptime computeSize()}`
```

```ds expected
const label = `size: ${comptime computeSize()}`;
```

### comptime block template value

Comptime block values expand inside template interpolations.

```ds
const label = `size: ${comptime { computeSize() }}`
```

```ds expected
const label = `size: ${
    comptime {
        computeSize()
    }
}`;
```

