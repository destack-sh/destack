# Range Types

Range type expressions use the same spelling as range values.

## bounds

### type range bounds

```ds
type Window = Start .. End
type Inclusive = Start ..= End
type From = Start ..
type To = .. End
type Full = ..
```

```ds expected
type Window = Start..End;
type Inclusive = Start..=End;
type From = Start..;
type To = ..End;
type Full = ..;
```
