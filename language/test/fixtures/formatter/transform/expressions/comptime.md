# Comptime Expressions

Tests for `comptime` expression formatting.

## Basic Comptime

### comptime expression

Comptime expressions keep `comptime` tight to the body.

```ds
const value = comptime 1 + 2 + 3
```

```ds expected
const value = comptime 1 + 2 + 3;
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

Comptime blocks in statement position keep the outer expression statement-valued.

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
