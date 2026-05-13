# Range Indexing

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
