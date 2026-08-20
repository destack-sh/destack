# Object Shorthand

## Shorthand Properties

### shorthand property

Shorthand properties use the variable name as both key and value.

```ds
const x = { a, b, c }
```

```ds expected
const x = { a, b, c };
```

### mixed shorthand and regular

Shorthand and regular properties can be mixed.

```ds
const x = { a, b: 2, c }
```

```ds expected
const x = { a, b: 2, c };
```

### shorthand with method

Objects with methods expand to multiple lines.

```ds
const x = { a, method() { return 1 } }
```

```ds expected
const x = {
    a,
    method() {
        return 1;
    },
};
```
