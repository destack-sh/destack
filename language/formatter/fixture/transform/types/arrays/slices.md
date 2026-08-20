# Slices

## Slices

### slice type alias

Slice type syntax has no separator after the element type.

```ds
type Values=[int32]
```

```ds expected
type Values = [int32];
```

### slice function parameter

Slice annotations format the same way in parameter and return positions.

```ds
function take(values:[string]):[string]{values}
```

```ds expected
function take(values: [string]): [string] {
    values
}
```
