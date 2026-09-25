# Const Blocks

## Const Blocks

### const block expression

Const blocks format like other blocks.

```tspp
const table = const { const x = 1; x + 1 }
```

```tspp expected
const table = const {
    const x = 1;
    x + 1
};
```

### const block function tail

Const blocks in function tail position preserve their final value.

```tspp
function table(): number { const { const value = buildTable(); value.size } }
```

```tspp expected
function table(): number {
    const {
        const value = buildTable();
        value.size
    }
}
```

### const block statement position

Const blocks in statement position keep the outer expression as a statement.

```tspp
function prepare(): void { const { const value = buildTable(); install(value) } }
```

```tspp expected
function prepare(): void {
    const {
        const value = buildTable();
        install(value)
    };
}
```

### const block with explicit terminal statement

Explicit semicolons inside const blocks are preserved.

```tspp
function table(): number { const { const value = buildTable(); value.size; } }
```

```tspp expected
function table(): number {
    const {
        const value = buildTable();
        value.size;
    }
}
```

### nested const block tail

Nested const blocks preserve their own block tail expression.

```tspp
function table(): number { const { const value = const { buildTable() }; value.size } }
```

```tspp expected
function table(): number {
    const {
        const value = const {
            buildTable()
        };
        value.size
    }
}
```
