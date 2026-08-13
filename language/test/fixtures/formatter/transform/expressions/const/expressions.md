# Const Expressions

## Const Forms

### const expression

Const expressions keep `const` tight to the body.

```ds
const value = const 1 + 2 + 3
```

```ds expected
const value = const 1 + 2 + 3;
```

### const operand values

Const values follow their inner expression shape in operand positions.

```ds
render(const buildView())
const value = (const createBuilder()).build()
```

```ds expected
render(const buildView());
const value = (const createBuilder()).build();
```

### const collection values

Const values keep spread and object-value grouping.

```ds
const values = [const buildValue(), ...(const buildValues())]
const envelope = { value: const buildValue() }
```

```ds expected
const values = [const buildValue(), ...const buildValues()];
const envelope = { value: const buildValue() };
```

### const parameter default value

Default parameters can use compact const values.

```ds
function render(view = const buildView()) { use(view) }
```

```ds expected
function render(view = const buildView()) {
    use(view)
}
```

### const template value

Compact const values stay inline inside template interpolations.

```ds
const label = `size: ${const computeSize()}`
```

```ds expected
const label = `size: ${const computeSize()}`;
```

### const block template value

Const block values expand inside template interpolations.

```ds
const label = `size: ${const { computeSize() }}`
```

```ds expected
const label = `size: ${
    const {
        computeSize()
    }
}`;
```

