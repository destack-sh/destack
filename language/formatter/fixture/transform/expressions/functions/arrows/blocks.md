# Arrow Function Blocks

## Arrow Functions with Block Bodies

### arrow function with block body

Block bodies expand to multiple lines with indented content.

```ds
const f = (x) => { return x * 2 }
```

```ds expected
const f = (x) => {
    return x * 2;
};
```

### arrow function with multiple statements

Multiple statements require a block body.

```ds
const f = (x) => { const y = x * 2; return y + 1 }
```

```ds expected
const f = (x) => {
    const y = x * 2;
    return y + 1;
};
```

### short arrow block with value tail

Short arrow blocks expand nested control-flow tails.

```ds
const f = (x: number): number => { const y = x * 2; if (y > 10) { y } else { y + 1 } }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    if (y > 10) {
        y
    } else {
        y + 1
    }
};
```

### expanded arrow block with value tail

Arrow blocks preserve semicolonless value tails.

```ds
const f = (x: number): number => { const y = x * 2; if (y > 10) { const capped = y - 1; capped } else { const boosted = y + 1; boosted } }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    if (y > 10) {
        const capped = y - 1;
        capped
    } else {
        const boosted = y + 1;
        boosted
    }
};
```

### arrow block with statement tail

Terminal semicolons in arrow blocks keep statement position.

```ds
const f = (x: number): number => { const y = x * 2; y; }
```

```ds expected
const f = (x: number): number => {
    const y = x * 2;
    y;
};
```

### empty block body

Empty blocks have internal spacing.

```ds
const f = () => { }
```

```ds expected
const f = () => {};
```
