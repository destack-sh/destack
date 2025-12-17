# Object Literals

Tests for object literal formatting.

## Spacing

### object literals have internal spacing

Object braces get internal spacing, unlike arrays.

```ds
const x = {a:1,b:2}
```

Spaces are added after `{` and before `}`.

```ds expected
const x = { a: 1, b: 2 };
```

### object property colon has trailing space only

Property colons have no space before and one space after.

```ds
const x = { a : 1 }
```

```ds expected
const x = { a: 1 };
```

### short objects stay on one line

Short object literals remain on a single line.

```ds
const x = { a: 1, b: 2 }
```

```ds expected
const x = { a: 1, b: 2 };
```
