# Const Conditions

## Const Conditions

### const if condition

Const conditions keep the keyword in the condition.

```tspp
if (const Flag) { configure() }
```

```tspp expected
if (const Flag) {
    configure()
}
```

### const if tail condition

Const if conditions preserve branch values when the if expression is a function tail.

```tspp
function choose(): number { if (const Flag) { one } else { two } }
```

```tspp expected
function choose(): number {
    if (const Flag) {
        one
    } else {
        two
    }
}
```

### const if operand values

Const if conditions follow if-expression branch formatting.

```tspp line-width=80
const value = if (const Flag) { buildPrimaryValue(context.locale, context.timeZone) } else { buildFallbackValue(context.locale, context.timeZone) }
const result = (if (const Flag) { createReadyBuilder(context) } else { createPendingBuilder(context) }).build().finalize()
```

```tspp expected
const value = if (const Flag) {
    buildPrimaryValue(context.locale, context.timeZone)
} else {
    buildFallbackValue(context.locale, context.timeZone)
};
const result = (if (const Flag) {
    createReadyBuilder(context)
} else {
    createPendingBuilder(context)
})
    .build()
    .finalize();
```

### const if statement condition

Const if conditions in void bodies keep branch tail expressions semicolonless.

```tspp
function choose(): void { if (const Flag) { one() } else { two() } }
```

```tspp expected
function choose(): void {
    if (const Flag) {
        one()
    } else {
        two()
    }
}
```
