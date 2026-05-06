# Regex Literals

Regex literals create `RegExp` values.

## regex literals

### regex literal yields RegExp

Regex literals are assignable to `RegExp`.

```ds
let value: RegExp = /abc/;
```

### regex literals are not strings

Regex literals are not assignable to `string`.

```ds
let value: string = /abc/;
```

- contains: not assignable

### regex literals expose RegExp members

Regex literals expose the declared `RegExp` API.

```ds
const value = /abc/;
const text = value.toString();
text satisfies string;
```

### regex literals work with RegExp unions

Regex literals can flow into unions that include `RegExp`.

```ds
let value: RegExp | int32 = /abc/;
value satisfies RegExp | int32;
```

### regex literals reject boolean contexts

Regex literals reject assignment to unrelated primitive types.

```ds
let value: boolean = /abc/;
```

- contains: not assignable
