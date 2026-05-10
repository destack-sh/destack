# Object Methods

## Methods

### method in object

Methods expand the object to multiple lines.

```ds
const x = { foo() { return 1 } }
```

```ds expected
const x = {
    foo() {
        return 1;
    },
};
```

### method with parameters

Method parameters follow function formatting rules.

```ds
const x = { add(a, b) { return a + b } }
```

```ds expected
const x = {
    add(a, b) {
        return a + b;
    },
};
```

### async method

Async methods use the `async` keyword before the name.

```ds
const x = { async fetch() { return await data } }
```

```ds expected
const x = {
    async fetch() {
        return await data;
    },
};
```

### generator method

Generator methods use `*` before the name.

```ds
const x = { *items() { yield 1; yield 2 } }
```

```ds expected
const x = {
    *items() {
        yield 1;
        yield 2;
    },
};
```

### getter

Getters use `get` keyword before the property name.

```ds
const x = { get value() { return this._value } }
```

```ds expected
const x = {
    get value() {
        return this._value;
    },
};
```

### setter

Setters use `set` keyword and take one parameter.

```ds
const x = { set value(v) { this._value = v } }
```

```ds expected
const x = {
    set value(v) {
        this._value = v;
    },
};
```

### method tail expression

Value-returning object methods keep terminal expressions semicolonless.

```ds
const x = { value(): number { this.current } }
```

```ds expected
const x = {
    value(): number {
        this.current
    },
};
```

### method short nested value tail

Short object methods expand nested control-flow tails.

```ds
const x = { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { this.limit } else { doubled } } }
```

```ds expected
const x = {
    value(next: number): number {
        const doubled = next * 2;
        if (doubled > this.limit) {
            this.limit
        } else {
            doubled
        }
    },
};
```

### method expanded nested value tail

Object methods preserve expression tails through nested control flow.

```ds
const x = { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { const capped = this.limit - 1; capped } else { const returned = doubled + 1; returned } } }
```

```ds expected
const x = {
    value(next: number): number {
        const doubled = next * 2;
        if (doubled > this.limit) {
            const capped = this.limit - 1;
            capped
        } else {
            const returned = doubled + 1;
            returned
        }
    },
};
```
