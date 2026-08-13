# Const Blocks

## Const Blocks

### const block expression

Const blocks format like other blocks.

```ds
const table = const { const x = 1; x + 1 }
```

```ds expected
const table = const {
    const x = 1;
    x + 1
};
```

### const block function tail

Const blocks in function tail position preserve their final value.

```ds
function table(): number { const { const value = buildTable(); value.size } }
```

```ds expected
function table(): number {
    const {
        const value = buildTable();
        value.size
    }
}
```

### const block statement position

Const blocks in statement position keep the outer expression as a statement.

```ds
function prepare(): void { const { const value = buildTable(); install(value) } }
```

```ds expected
function prepare(): void {
    const {
        const value = buildTable();
        install(value)
    };
}
```

### const block with explicit terminal statement

Explicit semicolons inside const blocks are preserved.

```ds
function table(): number { const { const value = buildTable(); value.size; } }
```

```ds expected
function table(): number {
    const {
        const value = buildTable();
        value.size;
    }
}
```

### nested const block tail

Nested const blocks preserve their own block tail expression.

```ds
function table(): number { const { const value = const { buildTable() }; value.size } }
```

```ds expected
function table(): number {
    const {
        const value = const {
            buildTable()
        };
        value.size
    }
}
```
