# Text

`string` keeps the JS-observable semantics in `.ds`: UTF-16 lengths, code point iteration, and `char` indexing.

## semantics

### length counts utf-16 code units

`length` matches JavaScript.

```ds
const text = "héllo";

text.length satisfies uint;
```

### iteration yields code points

`for-of` iterates by code point, as in JavaScript.

```ds
const text = "héllo";

for (const c of text) {
    c satisfies char;
}
```

### indexing yields chars

`.ds` indexing produces a scalar `char` instead of a one-element string.

```ds
const text = "héllo";

text[0] satisfies char;
```

### concatenation stays a string

`+` on strings concatenates.

```ds
const text = "hé" + "llo";

text satisfies string;
```
