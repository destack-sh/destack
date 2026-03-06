# Selection Range

## Partial Results

### Expand nested ranges

Selection range requests should return the nested parent chain for the selected expression.

```ds:main.ds
function sum(): number {
    return /*selection*/1 + 2;
}
```

```lsp selection_range
file=main.ds
range=1:11-1:12
parent=1:11-1:16
parent=1:4-1:16
parent=1:4-1:17
parent=0:23-2:1
parent=0:0-2:1
```
