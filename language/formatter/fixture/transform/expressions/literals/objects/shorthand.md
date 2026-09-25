# Object Shorthand

## Shorthand Properties

### shorthand property

Shorthand properties use the variable name as both key and value.

```tspp
const x = { a, b, c }
```

```tspp expected
const x = { a, b, c };
```

### mixed shorthand and regular

Shorthand and regular properties can be mixed.

```tspp
const x = { a, b: 2, c }
```

```tspp expected
const x = { a, b: 2, c };
```

### shorthand with method

Objects with methods expand to multiple lines.

```tspp
const x = { a, method() { return 1 } }
```

```tspp expected
const x = {
    a,
    method() {
        return 1;
    },
};
```
