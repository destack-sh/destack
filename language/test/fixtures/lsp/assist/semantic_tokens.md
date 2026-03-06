# Semantic Tokens

## Full Document

### Function and parameter tokens

Semantic tokens should classify function and parameter declarations across the full file.

```ds:main.ds
function greet(name: string): string {
    return name;
}
```

```lsp semantic_tokens
range=1:10-1:15
text=greet
kind=function
modifier=declaration

range=1:16-1:20
text=name
kind=parameter
modifier=declaration

range=1:22-1:28
text=string
kind=type

range=1:31-1:37
text=string
kind=type

range=2:12-2:16
text=name
kind=variable
```

## Range Requests

### Parameter and type tokens in range

Range requests should return only the tokens that fall inside the requested span.

```ds:main.ds
function greet([|name: string|]): string {
    return name;
}
```

```lsp semantic_tokens_range
range=1:16-1:20
text=name
kind=parameter
modifier=declaration

range=1:22-1:28
text=string
kind=type
```

## Delta Updates

### Comment shifts preserve token meaning

Semantic-token delta updates should rewrite the baseline token stream after a text shift.

```ds:main.ds
function greet(name: string): string {
   return name;
}
```

```lsp semantic_tokens_delta_source
// moved tokens
function greet(name: string): string {
    return name;
}
```

```lsp semantic_tokens_delta
range=2:10-2:15
text=greet
kind=function
modifier=declaration

range=2:16-2:20
text=name
kind=parameter
modifier=declaration

range=2:22-2:28
text=string
kind=type

range=2:31-2:37
text=string
kind=type

range=3:12-3:16
text=name
kind=variable
```
