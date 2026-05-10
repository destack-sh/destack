# Nested Arrow Functions

## Nested Arrow Functions

### nested arrow functions

Nested arrows each get parentheses added.

```ds
const f = x => y => x + y
```

```ds expected
const f = (x) => (y) => x + y;
```

### deeply nested arrow functions

Any depth of nesting gets consistent parentheses.

```ds
const f = a => b => c => a + b + c
```

```ds expected
const f = (a) => (b) => (c) => a + b + c;
```

### curried function with parens

Already-parenthesized curried functions are preserved.

```ds
const f = (a) => (b) => (c) => a + b + c
```

```ds expected
const f = (a) => (b) => (c) => a + b + c;
```
