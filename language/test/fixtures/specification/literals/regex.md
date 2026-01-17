# Regex Literals

Regex literals behave like string-like scalar literals.

## regex literals

### regex literal yields string

> Regex literals are string-like values.

```ds
let value: string = /abc/;
```

### regex literal is not assignable to number

> Regex literals are not assignable to number.

```ds
let value: number = /abc/;
```

- contains: not assignable
