
## Source Range

### Highlight identifiers inside a requested range

Only resolved identifiers inside the requested range are highlighted.

```ds main.ds
const first = 1;
      ^^^^^ first
const second = 2;
      ^^^^^^ second
```

```query semantic_tokens_range main.ds#second
@semantic_tokens_range.token range=main.ds#second type=variable modifiers=declaration,readonly
```

### Highlight every identifier inside a larger range

Highlights inside the range retain source order.

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

### Exclude identifiers crossing the requested range

An identifier is highlighted only when the requested range contains it completely.

```ds main.ds
const value = 1;
      ^^^^^ token
       ^^^ interior
```

```query semantic_tokens_range main.ds#interior
@semantic_tokens_range.none
```

### Highlight the selected identifier after an edit

The requested range uses the selected edit.

```ds main.ds
const first = 1;
const second = 2;
      ^^^^^^ second
```

```query semantic_tokens_range main.ds#second
@semantic_tokens_range.token range=main.ds#second type=variable modifiers=declaration,readonly
```

```ds main.ds change
const inserted = 0;
const first = 1;
const second = 2;
      ^^^^^^ second
```

```query semantic_tokens_range main.ds#second
@semantic_tokens_range.token range=main.ds#second type=variable modifiers=declaration,readonly
```

## Empty Results

### Preserve comment and literal highlighting

A range containing only comments and literals needs no additional highlighting.

```ds main.ds
// ordinary comment
"text";
^^^^^^^ lexical
```

```query semantic_tokens_range main.ds#lexical
@semantic_tokens_range.none
```
