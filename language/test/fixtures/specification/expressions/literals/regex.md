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

### regex literals expose string members

> Regex literals are string-like and expose string members.

```ds
const value = /abc/;
const length = value.length;
length satisfies int32;
```

### regex literals compose with string unions

> Regex literals can flow into unions that include string.

```ds
let value: string | int32 = /abc/;
value satisfies string | int32;
```

### regex literals reject boolean contexts

> Regex literals reject assignment to non string-like primitives.

```ds
let value: boolean = /abc/;
```

- contains: not assignable
