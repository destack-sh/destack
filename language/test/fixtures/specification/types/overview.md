# Type Overview

## nominal and structural types

### nominal newtypes are distinct

> Nominal newtypes do not mix even when they share a backing type.

```ds
newtype UserId = int64;
newtype OrderId = int64;
// UserId and OrderId don't mix, even though both are int64
```

### structs model value types

> Structs model value oriented types with named fields.

```ds
struct Point { x: float32; y: float32 }
```
