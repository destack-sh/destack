# Range Values

Range values use tight operator spacing.

## bounds

### value range bounds

Range operators are attached to their bounds.

```tspp
const halfOpen = 1 .. 10
const inclusive = 1 ..= 10
const from = 1 ..
const to = .. 10
const through = ..= 10
const full = ..
```

```tspp expected
const halfOpen = 1..10;
const inclusive = 1..=10;
const from = 1..;
const to = ..10;
const through = ..=10;
const full = ..;
```

## endpoints

### arithmetic endpoints

Arithmetic endpoints stay inside the range.

```tspp
const window = (start + 1) .. (end * 2)
const nested = (1 .. 4) + count
const negative = -3 .. 3
```

```tspp expected
const window = start + 1..end * 2;
const nested = (1..4) + count;
const negative = -3..3;
```

### range boundary comments

Comments around range operators keep readable operator boundaries.

```tspp
const window = start /* start */ .. /* end */ end
const inclusive = 0 /* min */ ..= /* max */ 255
const from = start /* start */ ..
const fromCommented = start .. /* open */
const to = .. /* end */ end
const full = .. /* all */
```

```tspp expected
const window = start /* start */ .. /* end */ end;
const inclusive = 0 /* min */ ..= /* max */ 255;
const from = start /* start */ ..;
const fromCommented = start .. /* open */;
const to = .. /* end */ end;
const full = .. /* all */;
```
