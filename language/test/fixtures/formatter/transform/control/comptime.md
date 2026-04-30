# Comptime Expressions

Comptime fixtures cover `comptime` conditions, blocks, and value-tail behavior.

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

### comptime block expression

Comptime blocks format like other blocks.

```ds
const table = comptime { const x = 1; x + 1 }
```

```ds expected
const table = comptime {
    const x = 1;
    x + 1
};
```

### comptime if condition

Comptime conditions keep the keyword in the condition.

```ds
if (comptime Flag) { configure() }
```

```ds expected
if (comptime Flag) {
    configure()
}
```

### comptime block function tail

Comptime blocks in function tail position preserve their final value.

```ds
function table(): number { comptime { const value = buildTable(); value.size } }
```

```ds expected
function table(): number {
    comptime {
        const value = buildTable();
        value.size
    }
}
```

### comptime block statement position

Comptime blocks in statement position keep the outer expression as a statement.

```ds
function prepare(): void { comptime { const value = buildTable(); install(value) } }
```

```ds expected
function prepare(): void {
    comptime {
        const value = buildTable();
        install(value)
    };
}
```

### comptime if tail condition

Comptime if conditions preserve branch values when the if expression is a function tail.

```ds
function choose(): number { if (comptime Flag) { one } else { two } }
```

```ds expected
function choose(): number {
    if (comptime Flag) {
        one
    } else {
        two
    }
}
```

### comptime if operand values

Comptime if conditions follow if-expression branch layout policy.

```ds line-width=80
const value = if (comptime Flag) { buildPrimaryValue(context.locale, context.timeZone) } else { buildFallbackValue(context.locale, context.timeZone) }
const result = (if (comptime Flag) { createReadyBuilder(context) } else { createPendingBuilder(context) }).build().finalize()
```

```ds expected
const value = if (comptime Flag) {
    buildPrimaryValue(context.locale, context.timeZone)
} else {
    buildFallbackValue(context.locale, context.timeZone)
};
const result = (if (comptime Flag) {
    createReadyBuilder(context)
} else {
    createPendingBuilder(context)
})
    .build()
    .finalize();
```

### comptime if statement condition

Comptime if conditions in void bodies keep branch tail expressions semicolonless.

```ds
function choose(): void { if (comptime Flag) { one() } else { two() } }
```

```ds expected
function choose(): void {
    if (comptime Flag) {
        one()
    } else {
        two()
    }
}
```

### comptime block with explicit terminal statement

Explicit semicolons inside comptime blocks are preserved.

```ds
function table(): number { comptime { const value = buildTable(); value.size; } }
```

```ds expected
function table(): number {
    comptime {
        const value = buildTable();
        value.size;
    }
}
```

### nested comptime block tail

Nested comptime blocks preserve their own block tail expression.

```ds
function table(): number { comptime { const value = comptime { buildTable() }; value.size } }
```

```ds expected
function table(): number {
    comptime {
        const value = comptime {
            buildTable()
        };
        value.size
    }
}
```
