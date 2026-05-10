# Computed Object Properties

## Computed Properties

### computed property name

Computed properties use brackets around the key expression.

```ds
const x = { [key]: value }
```

```ds expected
const x = { [key]: value };
```

### computed property with expression

Any expression can be used as a computed key.

```ds
const x = { [a + b]: value }
```

```ds expected
const x = { [a + b]: value };
```

### computed property with template literal

Template literals can be computed keys.

```ds
const x = { [`prefix_${name}`]: value }
```

```ds expected
const x = { [`prefix_${name}`]: value };
```

### multiple computed properties

Computed property names stay compact when they fit.

```ds
const obj = { [key]: value, [`prefix_${name}`]: data }
```

```ds expected
const obj = { [key]: value, [`prefix_${name}`]: data };
```
