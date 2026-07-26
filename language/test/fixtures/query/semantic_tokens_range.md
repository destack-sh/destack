# Semantic Tokens Range

## Source Range

### Restrict tokens to the requested range

The response contains only resolved identifiers inside the requested range.

```ds main.ds
const first = 1;
      ^^^^^ first
const second = 2;
      ^^^^^^ second
```

```query semantic_tokens_range main.ds#second
@semantic_tokens_range.token range=main.ds#second type=variable modifiers=declaration,readonly
```

### Return every token inside a larger range

Range queries preserve the complete token subsequence in source order.

```ds main.ds
function identity(value: int32): int32 {
    return value;
}

const result = identity(1);
^^^^^^^^^^^^^^^^^^^^^^^^^^^ line
      ^^^^^^ result
               ^^^^^^^^ function
```

```query semantic_tokens_range main.ds#line
@semantic_tokens_range.token range=main.ds#result type=variable modifiers=declaration,readonly
@semantic_tokens_range.token range=main.ds#function type=function
```

### Exclude tokens crossing the requested boundary

A token is returned only when the requested range contains the complete token.

```ds main.ds
const value = 1;
      ^^^^^ token
       ^^^ interior
```

```query semantic_tokens_range main.ds#interior
@semantic_tokens_range.none
```

## Empty Results

### Return no tokens for lexical source

A range containing only comments and literals has no semantic tokens.

```ds main.ds
// ordinary comment
"text";
^^^^^^^ lexical
```

```query semantic_tokens_range main.ds#lexical
@semantic_tokens_range.none
```
