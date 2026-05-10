# Object Spread

## Spread

### spread in object

Spread copies properties from another object.

```ds
const x = { ...other }
```

```ds expected
const x = { ...other };
```

### multiple spreads

Multiple spreads merge properties in order.

```ds
const x = { ...a, ...b, ...c }
```

```ds expected
const x = { ...a, ...b, ...c };
```

### spread between properties

Spread can appear between regular properties.

```ds
const x = { a: 1, ...middle, b: 2 }
```

```ds expected
const x = { a: 1, ...middle, b: 2 };
```

### spread with shorthand

Spread and shorthand properties can mix.

```ds
const x = { a, ...rest, b: 2 }
```

```ds expected
const x = { a, ...rest, b: 2 };
```
