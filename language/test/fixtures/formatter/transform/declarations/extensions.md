# Extension Declarations

Tests for Destack extension declaration formatting.

## Basic Extensions

### simple extension

Extensions use `for` to specify the type being extended.

```ds
extension for  Vector2  { }
```

Empty extension bodies stay on one line with internal spacing.

```ds expected
extension for Vector2 { }
```

### extension with implements

Extensions can implement traits for the extended type.

```ds
extension for  Vector2  implements  Add < Vector2 >  { }
```

Generic type arguments have no internal spacing.

```ds expected
extension for Vector2 implements Add<Vector2> { }
```
