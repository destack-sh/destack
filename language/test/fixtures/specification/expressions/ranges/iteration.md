# Range Iteration

Start-bounded ranges are iterable when their endpoint type implements `Step`.
Startless ranges do not have a first value, so they do not implement `Iterable`.

## step

### start-bounded ranges iterate

```ds
declare const range: Range<int>;

range satisfies Iterable<int>;
```

### startless ranges do not iterate

```ds
declare const range: RangeTo<int>;

range satisfies Iterable<int>;
```

- contains: does not satisfy
