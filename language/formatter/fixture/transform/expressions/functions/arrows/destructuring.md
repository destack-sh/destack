# Arrow Destructuring

## Destructuring Parameters

### arrow function with object destructuring

Object destructuring in params extracts named properties.

```tspp
const f = ({ x, y }) => x + y
```

```tspp expected
const f = ({ x, y }) => x + y;
```

### arrow function with array destructuring

Array destructuring extracts elements by position.

```tspp
const f = ([a, b]) => a + b
```

```tspp expected
const f = ([a, b]) => a + b;
```

### arrow function with renamed destructuring

Properties can be renamed during destructuring.

```tspp
const f = ({ x: a, y: b }) => a + b
```

```tspp expected
const f = ({ x: a, y: b }) => a + b;
```

### arrow function with default values

Default values use `=` with surrounding spaces.

```tspp
const f = (x = 1) => x
```

```tspp expected
const f = (x = 1) => x;
```

### arrow function with rest parameters

Rest parameters collect remaining arguments into an array.

```tspp
const f = (...args) => args
```

```tspp expected
const f = (...args) => args;
```
