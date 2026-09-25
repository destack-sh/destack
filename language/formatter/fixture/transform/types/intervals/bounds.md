# Interval Bounds

Interval type expressions use range spelling in type position.

## bounds

### interval bounds

```tspp
type Window = Start .. End
type Inclusive = Start ..= End
type From = Start ..
type To = .. End
```

```tspp expected
type Window = Start..End;
type Inclusive = Start..=End;
type From = Start..;
type To = ..End;
```

### interval bound comments

Comments around interval bounds stay attached to the corresponding range side.

```tspp
type Window = Start /* start */ .. /* end */ End
type Inclusive = 0 /* min */ ..= /* max */ 255
type From = Start .. /* open */
type Full = .. /* all */
```

```tspp expected
type Window = Start /* start */ .. /* end */ End;
type Inclusive = 0 /* min */ ..= /* max */ 255;
type From = Start .. /* open */;
type Full = .. /* all */;
```
