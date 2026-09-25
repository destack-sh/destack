# Const Expressions

## Const Forms

### const expression

Const expressions keep `const` tight to the body.

```tspp
const value = const 1 + 2 + 3
```

```tspp expected
const value = const 1 + 2 + 3;
```

### const operand values

Const values follow their inner expression shape in operand positions.

```tspp
render(const buildView())
const value = (const createBuilder()).build()
```

```tspp expected
render(const buildView());
const value = (const createBuilder()).build();
```

### const collection values

Const values keep spread and object-value grouping.

```tspp
const values = [const buildValue(), ...(const buildValues())]
const envelope = { value: const buildValue() }
```

```tspp expected
const values = [const buildValue(), ...const buildValues()];
const envelope = { value: const buildValue() };
```

### const parameter default value

Default parameters can use compact const values.

```tspp
function render(view = const buildView()) { use(view) }
```

```tspp expected
function render(view = const buildView()) {
    use(view)
}
```

### const template value

Compact const values stay inline inside template interpolations.

```tspp
const label = `size: ${const computeSize()}`
```

```tspp expected
const label = `size: ${const computeSize()}`;
```

### const block template value

Const block values expand inside template interpolations.

```tspp
const label = `size: ${const { computeSize() }}`
```

```tspp expected
const label = `size: ${
    const {
        computeSize()
    }
}`;
```

