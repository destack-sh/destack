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

### Expand nested arithmetic ranges

Selection range requests should walk from the leaf identifier through the nested arithmetic parents.

```ds:main.ds
function compute(value: int32): int32 {
    return (/*selection*/value + 1) * 2;
}
```

```lsp selection_range
file=main.ds
range=1:12-1:17
parent=1:12-1:21
parent=1:11-1:22
parent=1:11-1:26
parent=1:4-1:26
parent=1:4-1:27
parent=0:38-2:1
parent=0:0-2:1
```

### Expand type annotation ranges

Selection range requests should expand from a type name through the enclosing declaration.

```ds:main.ds
struct Point {
    x: int32
    y: int32
}

const current: /*selection*/Point = Point { x: 1, y: 2 };
```

```lsp selection_range
file=main.ds
range=5:15-5:20
parent=5:6-5:43
parent=5:0-5:43
parent=5:0-5:44
```

### Expand member access call ranges

Selection range requests should walk from a member access through the call and enclosing statement.

```ds:main.ds
struct User {
    name: string
}

function greet(user: User): void {
    print(user./*selection*/name);
}
```

```lsp selection_range
file=main.ds
range=5:10-5:19
parent=5:4-5:20
parent=5:4-5:21
parent=4:33-6:1
parent=4:0-6:1
```

### Expand ranges after malformed syntax

Selection range requests should still work for later valid expressions in damaged files.

```ds:main.ds
broken(,

const value = /*selection*/1;
```

```lsp selection_range
file=main.ds
range=2:14-2:15
parent=2:6-2:15
parent=2:0-2:15
parent=2:0-2:16
```
