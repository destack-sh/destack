# Try Values

## Try Values

### try initializer value

Try initializer values with catch clauses expand branch blocks.

```tspp
const payload = try { readPayload(source) } catch (error) { recoverPayload(error) }
```

```tspp expected
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
};
```

### try object property value

Try object property values with catch clauses expand branch blocks.

```tspp
const envelope = { payload: try { readPayload(source) } catch (error) { recoverPayload(error) } }
```

```tspp expected
const envelope = {
    payload: try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
};
```

### try argument value

Try argument values with catch clauses expand branch blocks.

```tspp
render(try { readPayload(source) } catch (error) { recoverPayload(error) })
```

```tspp expected
render(
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
);
```

### try collection values

Try values keep required grouping in collection and spread positions.

```tspp
const values = [try { readPayload(source) } catch (error) { recoverPayload(error) }, ...(try { readMany(source) } catch (error) { [] })]
const envelope = { payload: try { readPayload(source) } catch (error) { recoverPayload(error) } }
```

```tspp expected
const values = [
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
    ...(try {
        readMany(source)
    } catch (error) {
        []
    }),
];
const envelope = {
    payload: try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
};
```

### try parameter default value

Default parameters can use expanded try values.

```tspp
function render(payload = try { readPayload(source) } catch (error) { recoverPayload(error) }) { use(payload) }
```

```tspp expected
function render(
    payload = try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
) {
    use(payload)
}
```

### try template value

Expanded try values indent inside template interpolations.

```tspp
const label = `payload: ${try { readPayload(source) } catch (error) { recoverPayload(error) }}`
```

```tspp expected
const label = `payload: ${
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    }
}`;
```

### try await operand value

Try await operands with catch clauses expand branch blocks.

```tspp
const awaited = await (try { load(source) } catch (error) { recover(error) })
```

```tspp expected
const awaited = await (try {
    load(source)
} catch (error) {
    recover(error)
});
```

### long try initializer value

Long try initializer values expand branch bodies.

```tspp line-width=80
const payload = try { const raw = readCachedPayload(cacheKey, options); parsePayload(raw, schema, options) } catch (error) { const diagnostic = diagnostics.describe(error, context.locale); recoverPayload(diagnostic, fallbackPayload, options) }
```

```tspp expected
const payload = try {
    const raw = readCachedPayload(cacheKey, options);
    parsePayload(raw, schema, options)
} catch (error) {
    const diagnostic = diagnostics.describe(error, context.locale);
    recoverPayload(diagnostic, fallbackPayload, options)
};
```

### long try await operand value

Long try await operands expand branch bodies.

```tspp line-width=80
const awaited = await (try { loadAsync(source) } catch (error) { recoverAsync(error) })
```

```tspp expected
const awaited = await (try {
    loadAsync(source)
} catch (error) {
    recoverAsync(error)
});
```

### multiline try initializer value

Manually broken try initializer values keep the expanded branch shape.

```tspp
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
}
```

```tspp expected
const payload = try {
    readPayload(source)
} catch (error) {
    recoverPayload(error)
};
```

### multiline try argument value

Manually broken try argument values keep the expanded branch shape.

```tspp
render(try {
    readPayload(source)
} catch (error) { recoverPayload(error) })
```

```tspp expected
render(
    try {
        readPayload(source)
    } catch (error) {
        recoverPayload(error)
    },
);
```
