# Address Spaces

Tests for address space annotations on references.

## borrowed references

### addrspace decorator applies to borrowed reference

> Address spaces can be specified on borrowed references.

```ds
struct Point {
    x: int32;
    y: int32;
}

function kernel(data: @addrspace("shared") &Point): int32 {
    data.x
}
```
