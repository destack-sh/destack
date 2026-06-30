# Range Subscripts

Range subscripts use index-expression spacing.

## slices

### range indexing

```ds
const middle = items[ 1 .. count ]
const through = items[ 1 ..= count ]
const tail = items[ start .. ]
const head = items[ .. end ]
const prefix = items[ ..= end ]
const all = items[ .. ]
```

```ds expected
const middle = items[1..count];
const through = items[1..=count];
const tail = items[start..];
const head = items[..end];
const prefix = items[..=end];
const all = items[..];
```

### range indexing boundary comments

Comments around range index operators keep readable operator boundaries.

```ds
const middle = items[start /* start */ .. /* end */ end]
const head = items[.. /* end */ end]
const tail = items[start /* start */ ..]
const all = items[.. /* all */]
```

```ds expected
const middle = items[start /* start */ .. /* end */ end];
const head = items[.. /* end */ end];
const tail = items[start /* start */ ..];
const all = items[.. /* all */];
```
