# Nominal Interfaces

Tests for `newtype interface` nominal behavior.

## assignment

### nominal interface requires explicit implements

> Structural matches do not satisfy nominal interfaces.

```ds
newtype interface Add<T> {
    add(other: T): T;
}

struct Vec2 {
    x: int32,
    y: int32,

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
```

- type Vec2 is not assignable to type Add<Vec2>

### nominal interface satisfied via implements

> Explicit implements provides nominal conformance.

```ds
newtype interface Add<T> {
    add(other: T): T;
}

struct Vec2 {
    x: int32,
    y: int32,
}

extension for Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```
