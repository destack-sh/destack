# Object Spread

## Spread

### spread in object

Spread copies properties from another object.

```tspp
const x = { ...other }
```

```tspp expected
const x = { ...other };
```

### multiple spreads

Multiple spreads merge properties in order.

```tspp
const x = { ...a, ...b, ...c }
```

```tspp expected
const x = { ...a, ...b, ...c };
```

### spread between properties

Spread can appear between regular properties.

```tspp
const x = { a: 1, ...middle, b: 2 }
```

```tspp expected
const x = { a: 1, ...middle, b: 2 };
```

### spread with shorthand

Spread and shorthand properties can mix.

```tspp
const x = { a, ...rest, b: 2 }
```

```tspp expected
const x = { a, ...rest, b: 2 };
```
