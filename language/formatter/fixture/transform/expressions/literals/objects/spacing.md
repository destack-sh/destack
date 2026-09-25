# Object Literals

## Spacing

### object literals have internal spacing

Object braces get internal spacing, unlike arrays.

```tspp
const x = {a:1,b:2}
```

Spaces are added after `{` and before `}`.

```tspp expected
const x = { a: 1, b: 2 };
```

### object property colon has trailing space only

Property colons have no space before and one space after.

```tspp
const x = { a : 1 }
```

```tspp expected
const x = { a: 1 };
```

### short objects stay on one line

Short object literals remain on a single line.

```tspp
const x = { a: 1, b: 2 }
```

```tspp expected
const x = { a: 1, b: 2 };
```

### empty object

Empty objects stay compact.

```tspp
const x = {  }
```

```tspp expected
const x = {};
```

### trailing comma

Trailing commas in source are normalized.

```tspp
const obj = { a: 1, }
```

```tspp expected
const obj = { a: 1 };
```

### single property object

Single properties have spacing normalized.

```tspp
const x = {   a : 1   }
```

```tspp expected
const x = { a: 1 };
```
