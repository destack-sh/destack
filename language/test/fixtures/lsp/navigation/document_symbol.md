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

