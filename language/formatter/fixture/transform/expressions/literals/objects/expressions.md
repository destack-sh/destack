# Object Expressions

## Object Expressions

### config object pattern

Config objects expand to multiple lines when exceeding width.

```ds line-width=50
const config = { host: "localhost", port: 3000, debug: true, timeout: 5000 }
```

```ds expected
const config = {
    host: "localhost",
    port: 3000,
    debug: true,
    timeout: 5000,
};
```

### object with mixed content

Mixed properties and methods expand to multiple lines.

```ds line-width=40
const x = { name: "test", items: [1, 2], handler() { return this.name } }
```

```ds expected
const x = {
    name: "test",
    items: [1, 2],
    handler() {
        return this.name;
    },
};
```

### object as argument

Objects can be passed directly as function arguments.

```ds
foo({ a: 1, b: 2 })
```

```ds expected
foo({ a: 1, b: 2 });
```
