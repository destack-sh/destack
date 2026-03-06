# Workspace Symbol

## Member Search

### Search by member name

Workspace symbol search should return the expected member matches for the query string.

```ds:main.ds
[|class /*logger*/Logger {
    [|/*log*/log(message: string): void {}|]
    [|level: int32|]
}|]
```

```lsp workspace_symbol_query
log
```

```lsp workspace_symbol
log|method|Logger
logger|class
```

