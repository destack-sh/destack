# Object Literals

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

### empty object

Empty objects stay compact.

```ds
const x = {  }
```

```ds expected
const x = {};
```

### trailing comma

Trailing commas in source are normalized.

```ds
const obj = { a: 1, }
```

```ds expected
const obj = { a: 1 };
```

### single property object

Single properties have spacing normalized.

```ds
const x = {   a : 1   }
```

```ds expected
const x = { a: 1 };
```
