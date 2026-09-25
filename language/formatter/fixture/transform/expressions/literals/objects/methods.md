# Object Methods

## Methods

### method in object

Methods expand the object to multiple lines.

```tspp
const x = { foo() { return 1 } }
```

```tspp expected
const x = {
    foo() {
        return 1;
    },
};
```

### method with parameters

Method parameters follow function formatting rules.

```tspp
const x = { add(a, b) { return a + b } }
```

```tspp expected
const x = {
    add(a, b) {
        return a + b;
    },
};
```

### async method

Async methods use the `async` keyword before the name.

```tspp
const x = { async fetch() { return await data } }
```

```tspp expected
const x = {
    async fetch() {
        return await data;
    },
};
```

### generator method

Generator methods use `*` before the name.

```tspp
const x = { *items() { yield 1; yield 2 } }
```

```tspp expected
const x = {
    *items() {
        yield 1;
        yield 2;
    },
};
```

### getter

Getters use `get` keyword before the property name.

```tspp
const x = { get value() { return this._value } }
```

```tspp expected
const x = {
    get value() {
        return this._value;
    },
};
```

### setter

Setters use `set` keyword and take one parameter.

```tspp
const x = { set value(v) { this._value = v } }
```

```tspp expected
const x = {
    set value(v) {
        this._value = v;
    },
};
```

### method tail expression

Value-returning object methods keep terminal expressions semicolonless.

```tspp
const x = { value(): number { this.current } }
```

```tspp expected
const x = {
    value(): number {
        this.current
    },
};
```

### method short nested value tail

Short object methods expand nested control-flow tails.

```tspp
const x = { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { this.limit } else { doubled } } }
```

```tspp expected
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

```tspp
const x = { value(next: number): number { const doubled = next * 2; if (doubled > this.limit) { const capped = this.limit - 1; capped } else { const returned = doubled + 1; returned } } }
```

```tspp expected
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
