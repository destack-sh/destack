# Interval Bounds

Interval type expressions use range spelling in type position.

## bounds

### interval bounds

```ds
type Window = Start .. End
type Inclusive = Start ..= End
type From = Start ..
type To = .. End
```

```ds expected
type Window = Start..End;
type Inclusive = Start..=End;
type From = Start..;
type To = ..End;
```
