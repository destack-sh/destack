# Where

> NOTE #Incomplete: implement/mdtest where clauses

Readable generic constraints beyond inline syntax.

## Coverage

- **Single constraint**: `where T: Copy`
- **Multiple constraints**: `where (T: Mergeable, U: Comparable)`
- **Complex bounds**: Multi-line constraint lists

## Example

```ds
function process<T>(x: T): T where T: Copy {
    // ...
}

function merge<T, U>(): T where (
    T: Mergeable,
    U: Comparable<T>
) {
    // ...
}
```
