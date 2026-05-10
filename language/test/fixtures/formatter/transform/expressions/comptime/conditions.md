# Comptime Conditions

## Comptime Conditions

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

Comptime if conditions follow if-expression branch formatting.

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
