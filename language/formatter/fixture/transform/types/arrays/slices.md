# Slices

## Slices

### slice type alias

Slice type syntax has no separator after the element type.

```tspp
type Values=[int32]
```

```tspp expected
type Values = [int32];
```

### slice function parameter

Slice annotations format the same way in parameter and return positions.

```tspp
function take(values:[string]):[string]{values}
```

```tspp expected
function take(values: [string]): [string] {
    values
}
```
