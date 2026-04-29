# Pattern Integration

Pattern integration fixtures cover nested patterns in value-producing control forms.

## Match Arms

### nested destructuring guard with block tail

Nested tagged, array, and object patterns compose with guards and block tail expressions.

```ds line-width=120
const score = match (packet) { Event.Batch([first, ..., last], { meta: { id = fallbackId }, ...rest }) if (rest.valid) => { log(id); first + last + id }; Event.Single(Point { x, y }) => x + y; _ => 0 }
```

```ds expected
const score = match (packet) {
    Event.Batch(
        [first, ..., last],
        {
            meta: { id = fallbackId },
            ...rest
        },
    ) if (rest.valid) => {
        log(id);
        first + last + id
    }
    Event.Single(Point { x, y }) => x + y
    _ => 0
};
```

### catch match with nested patterns and comments

Catch-match patterns keep comments attached while preserving arm value tails.

```ds
try { read() } catch match (error) { // network
Network.Timeout { duration } if (duration > 1000) => retry(duration); // validation
Validation.Errors([first, ...rest]) => { report(first, rest); fallback() }; _ => throw error }
```

```ds expected
try {
    read()
} catch match (error) {
    // network
    Network.Timeout { duration } if (duration > 1000) => retry(duration)
    // validation
    Validation.Errors([first, ...rest]) => {
        report(first, rest);
        fallback()
    }
    _ => throw error
}
```
