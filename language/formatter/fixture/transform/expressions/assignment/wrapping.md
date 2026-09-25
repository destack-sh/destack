# Assignment Wrapping

Assignment wrapping fixtures cover assignment chains and long assignment targets.

## Assignments and Chains

### assignment with chained call breaks at the chain

When the right hand side is a chain, prefer breaking at the chain segments.

```tspp line-width=40
const result = someVeryLongChain().a().b().c()
```

```tspp expected
const result = someVeryLongChain()
    .a()
    .b()
    .c();
```

### assignment with long left hand side still breaks at the chain

Even with a long left hand side, the chain should break cleanly.

```tspp line-width=50
const veryLongResultName = someVeryLongChain().a().b().c().d()
```

```tspp expected
const veryLongResultName = someVeryLongChain()
    .a()
    .b()
    .c()
    .d();
```

## Chained Assignments

### chained assignment

Multiple assignments in one expression.

```tspp
a = b = c = 1
```

```tspp expected
a = b = c = 1;
```

### long chained assignment breaks by assignment depth

Chained assignments keep right associativity while breaking by assignment depth.

```tspp line-width=30
veryLongName = anotherLongName = thirdLongName = 42
```

```tspp expected
veryLongName =
    anotherLongName =
    thirdLongName =
        42;
```
