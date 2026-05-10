# Arrow Destructuring

## Destructuring Parameters

### arrow function with object destructuring

Object destructuring in params extracts named properties.

```ds
const f = ({ x, y }) => x + y
```

```ds expected
const f = ({ x, y }) => x + y;
```

### arrow function with array destructuring

Array destructuring extracts elements by position.

```ds
const f = ([a, b]) => a + b
```

```ds expected
const f = ([a, b]) => a + b;
```

### arrow function with renamed destructuring

Properties can be renamed during destructuring.

```ds
const f = ({ x: a, y: b }) => a + b
```

```ds expected
const f = ({ x: a, y: b }) => a + b;
```

### arrow function with default values

Default values use `=` with surrounding spaces.

```ds
const f = (x = 1) => x
```

```ds expected
const f = (x = 1) => x;
```

### arrow function with rest parameters

Rest parameters collect remaining arguments into an array.

```ds
const f = (...args) => args
```

```ds expected
const f = (...args) => args;
```
