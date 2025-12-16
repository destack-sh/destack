# Completion

## Struct Fields

### Complete struct fields after dot

```ds
struct Point {
    x: float32,
    y: float32,
}

extension for Point {
    magnitude(): float32 {
        (this.x * this.x + this.y * this.y).sqrt()
    }
}

const p = Point { x: 1, y: 2 };
p.$0
```

```query completion $0
- x: field
- y: field
- magnitude: method
```

### Complete nested struct fields

```ds
struct Inner { value: int32 }
struct Outer { inner: Inner }

const o = Outer { inner: Inner { value: 1 } };
o.inner.$0
```

```query completion $0
- value: field
```
