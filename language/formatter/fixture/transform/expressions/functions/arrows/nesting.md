# Nested Arrow Functions

## Nested Arrow Functions

### nested arrow functions

Nested arrows each get parentheses added.

```tspp
const f = x => y => x + y
```

```tspp expected
const f = (x) => (y) => x + y;
```

### deeply nested arrow functions

Any depth of nesting gets consistent parentheses.

```tspp
const f = a => b => c => a + b + c
```

```tspp expected
const f = (a) => (b) => (c) => a + b + c;
```

### curried function with parens

Already-parenthesized curried functions are preserved.

```tspp
const f = (a) => (b) => (c) => a + b + c
```

```tspp expected
const f = (a) => (b) => (c) => a + b + c;
```
