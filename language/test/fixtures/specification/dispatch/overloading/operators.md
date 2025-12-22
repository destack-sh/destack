# Operator Overloading

Tests for operator overloading via interface implementations.

## Addition

### extension implements Add

> The `+` operator dispatches to the `add` method on the receiver.

```ds
struct Vector2 { x: number, y: number }

extension for Vector2 implements Add<Vector2> {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();
const sum = left + right;
sum satisfies Vector2;
```

### extension without interface does not overload

> The `+` operator requires an explicit `implements Add` clause.

```ds
struct Vector2 { x: number, y: number }

extension for Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const left = getVector();
const right = getVector();
left + right;
```

- contains: no matching overload
