# Assignment Layout

Assignment layout fixtures cover assignment chains and long assignment targets.

## Assignments and Chains

### assignment with chained call breaks at the chain

When the right hand side is a chain, prefer breaking at the chain segments.

```ds line-width=40
const result = someVeryLongChain().a().b().c()
```

```ds expected
const result = someVeryLongChain()
    .a()
    .b()
    .c();
```

### assignment with long left hand side still breaks at the chain

Even with a long left hand side, the chain should break cleanly.

```ds line-width=50
const veryLongResultName = someVeryLongChain().a().b().c().d()
```

```ds expected
const veryLongResultName = someVeryLongChain()
    .a()
    .b()
    .c()
    .d();
```

## Chained Assignments

### chained assignment

Multiple assignments in one expression.

```ds
a = b = c = 1
```

```ds expected
a = b = c = 1;
```

### long chained assignment breaks by assignment depth

Chained assignments keep right associativity while breaking by assignment depth.

```ds line-width=30
veryLongName = anotherLongName = thirdLongName = 42
```

```ds expected
veryLongName =
    anotherLongName =
    thirdLongName =
        42;
```
