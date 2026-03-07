# Document Symbol

## Class Outlines

### Flatten nested class members

Document symbols should flatten the class outline into the expected ordered entries.

```ds:main.ds
[|class /*person*/Person {
    [|/*name*/name: string|]
    [|/*age*/age: int32|]

    [|/*greet*/greet(): string {
        return "Hello, " + this.name;
    }|]
}|]
```

```lsp document_symbol
person|class
name|field
age|field
greet|method
```

## Document outlines

### Drop a deleted method from the outline
Document symbols should shrink after one member is removed from the class.

```ds:main.ds
[|class /*person*/Person {
   [|/*name*/name: string|]

   [|/*greet*/greet(): string {
       return this.name;
   }|]
}|]
```

```ds:main.ds[1]
[|class /*person*/Person {
   [|/*name*/name: string|]
}|]
```

```lsp document_symbols main.ds [0]
person|class
name|field
greet|method
```

```lsp document_symbols main.ds [1]
person|class
name|field
```

### Grow the outline after adding a second class
Document symbols should include newly inserted sibling declarations in source order.

```ds:main.ds
[|class /*person*/Person {
   [|/*name*/name: string|]
}|]
```

```ds:main.ds[1]
[|class /*person*/Person {
   [|/*name*/name: string|]
}|]

[|class /*address*/Address {
   [|/*street*/street: string|]
}|]
```

```lsp document_symbols main.ds [0]
person|class
name|field
```

```lsp document_symbols main.ds [1]
person|class
name|field
address|class
street|field
```

### Reorder the outline after inserting a new top level declaration
Document symbols should follow source order after a new declaration is inserted before the old one.

```ds:main.ds
[|class /*person*/Person {
   [|/*name*/name: string|]
}|]
```

```ds:main.ds[1]
[|class /*address*/Address {
   [|/*street*/street: string|]
}|]

[|class /*person*/Person {
   [|/*name*/name: string|]
}|]
```

```lsp document_symbols main.ds [0]
person|class
name|field
```

```lsp document_symbols main.ds [1]
address|class
street|field
person|class
name|field
```

### Grow, reorder, and prune one outline across four states
Document symbols should track each intermediate outline shape instead of collapsing to the final state.

```ds:main.ds
[|class /*person*/Person {
   [|/*name*/name: string|]
}|]
```

```ds:main.ds[1]
[|class /*person*/Person {
   [|/*name*/name: string|]

   [|/*greet*/greet(): string {
       return this.name;
   }|]
}|]
```

```ds:main.ds[2]
[|class /*address*/Address {
   [|/*street*/street: string|]
}|]

[|class /*person*/Person {
   [|/*name*/name: string|]

   [|/*greet*/greet(): string {
       return this.name;
   }|]
}|]
```

```ds:main.ds[3]
[|class /*address*/Address {
   [|/*street*/street: string|]
}|]
```

```lsp document_symbols main.ds [0]
person|class
name|field
```

```lsp document_symbols main.ds [1]
person|class
name|field
greet|method
```

```lsp document_symbols main.ds [2]
address|class
street|field
person|class
name|field
greet|method
```

```lsp document_symbols main.ds [3]
address|class
street|field
```
