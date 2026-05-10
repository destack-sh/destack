# Comptime Blocks

## Comptime Blocks

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
