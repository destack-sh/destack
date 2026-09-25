# Fixed Arrays

## Fixed Arrays

### fixed array type alias

Fixed array type syntax spaces the element and length separator.

```tspp
type Pair=[int32;2]
```

```tspp expected
type Pair = [int32; 2];
```

### nested fixed array type alias

Nested fixed arrays keep each length with its own element type.

```tspp
type Matrix=[[int32;2];2]
```

```tspp expected
type Matrix = [[int32; 2]; 2];
```
