# Dynamic Arrays

## Dynamic Arrays

### dynamic array type alias

Dynamic array type syntax keeps the element type tight to the brackets.

```tspp
type Values= int32[]
```

```tspp expected
type Values = int32[];
```

### nested dynamic array type alias

Nested dynamic array suffixes stay compact.

```tspp
type Matrix= string[][]
```

```tspp expected
type Matrix = string[][];
```
