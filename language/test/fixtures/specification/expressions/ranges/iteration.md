# Range Iteration

Start-bounded ranges are iterable when their endpoint type implements `Step`.
Startless ranges do not have a first value, so they do not implement `Iterable`.

## step

### start-bounded ranges iterate

A start plus a `Step` endpoint type makes a range iterable.

```ds
declare const range: Range<int>;

range satisfies Iterable<int>;
```

### startless ranges do not iterate

Without a first value there is nothing to iterate.

```ds
declare const range: RangeTo<int>;

range satisfies Iterable<int>;
```

- contains: does not satisfy
